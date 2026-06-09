#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(not(target_family = "wasm"))]
pub use native::McpIntegration;
#[cfg(not(target_family = "wasm"))]
mod oauth;
#[cfg(not(target_family = "wasm"))]
mod utils;
#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(all(test, not(target_family = "wasm")))]
mod utils_tests;

use std::collections::HashMap;
#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

#[cfg(not(target_family = "wasm"))]
use diesel::SqliteConnection;
use futures_util::stream::AbortHandle;
#[cfg(not(target_family = "wasm"))]
use parking_lot::Mutex;
use uuid::Uuid;
use warpui::{Entity, SingletonEntity};

#[cfg(not(target_family = "wasm"))]
use crate::ai::mcp::templatable::CloudTemplatableMCPServer;
use crate::ai::mcp::templatable_installation::TemplatableMCPServerInstallation;
use crate::ai::mcp::MCPServerState;

/// Singleton model to manage state of MCP server lifecycles and panes across multiple windows
/// (where only one MCP server pane can exist per window).
///
/// Specifically:
/// - Maintains MCP server view handles to preserve state when panes are hidden
/// - Tracks currently open MCP server panes and their location
///
/// The core implementations are in the `native` and `wasm` modules.
#[derive(Default)]
pub struct TemplatableMCPServerManager {
    #[cfg(not(target_family = "wasm"))]
    cloud_templatable_mcp_servers: HashMap<Uuid, CloudTemplatableMCPServer>,
    locally_installed_servers: HashMap<Uuid, TemplatableMCPServerInstallation>,
    server_states: HashMap<Uuid, MCPServerState>,
    active_servers: HashMap<Uuid, TemplatableMCPServerInfo>,

    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    spawned_servers: HashMap<Uuid, SpawnedServerInfo>,
    /// Cached credentials for each server.
    ///
    /// We persist these to secure storage, and if they are present when the server is started,
    /// we use them instead of going through the OAuth flow again.
    #[cfg(not(target_family = "wasm"))]
    server_credentials: oauth::PersistedCredentialsMap,
    /// Cached credentials for file-based servers, keyed by installation hash.
    #[cfg(not(target_family = "wasm"))]
    file_based_server_credentials: oauth::FileBasedPersistedCredentialsMap,
    #[cfg(not(target_family = "wasm"))]
    database_connection: Option<Arc<Mutex<SqliteConnection>>>,
    /// Error messages for failed servers, keyed by installation UUID.
    server_error_messages: HashMap<Uuid, String>,
    /// Maps the OAuth CSRF `state` token to the installation UUID of the server whose
    /// authorization flow is in progress.
    ///
    /// Populated just before opening the authorization URL; removed once the callback
    /// is received or the spawn task terminates.
    #[cfg(not(target_family = "wasm"))]
    pending_oauth_csrf: HashMap<String, Uuid>,
}

/// Information about a spawned server task.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
struct SpawnedServerInfo {
    abort_handle: AbortHandle,
    #[cfg(not(target_family = "wasm"))]
    oauth_result_tx: async_channel::Sender<oauth::CallbackResult>,
}

/// Information about a single connected MCP server.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub struct TemplatableMCPServerInfo {
    service: rmcp::service::RunningService<
        rmcp::RoleClient,
        Box<dyn rmcp::service::DynService<rmcp::RoleClient>>,
    >,
    tools: Vec<rmcp::model::Tool>,
    /// Whether the underlying transport uses authentication.
    ///
    /// TODO(vorporeal): Use this to display a toast when server authentication and connection is complete, and
    /// to provide a "log out" button.
    #[allow(dead_code)]
    is_authenticated_transport: bool,
}


/// The current status of the Figma MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigmaMcpStatus {
    /// The Figma MCP server is not installed.
    NotInstalled,
    /// The Figma MCP server is installed but not currently running.
    Installed,
    /// The Figma MCP server is in the process of enabling (e.g. OAuth flow in progress).
    Enabling,
    /// The Figma MCP server is running.
    Running,
}

impl TemplatableMCPServerManager {
    pub fn get_installed_templatable_servers(
        &self,
    ) -> &HashMap<Uuid, TemplatableMCPServerInstallation> {
        &self.locally_installed_servers
    }

    pub fn get_installed_server(
        &self,
        installation_uuid: &Uuid,
    ) -> Option<&TemplatableMCPServerInstallation> {
        self.locally_installed_servers.get(installation_uuid)
    }

    /// Returns the UUID of the locally-installed Figma MCP server installation, if any.
    pub fn get_figma_installation_uuid(&self) -> Option<Uuid> {
        self.locally_installed_servers
            .iter()
            .find(|(_, installation)| {
                installation
                    .template_json()
                    .contains("https://mcp.figma.com/mcp")
            })
            .map(|(uuid, _)| *uuid)
    }

    /// Returns the current status of the Figma MCP server.
    pub fn get_figma_mcp_status(&self) -> FigmaMcpStatus {
        let Some(uuid) = self.get_figma_installation_uuid() else {
            return FigmaMcpStatus::NotInstalled;
        };
        if self.active_servers.contains_key(&uuid) {
            FigmaMcpStatus::Running
        } else if self.spawned_servers.contains_key(&uuid) {
            FigmaMcpStatus::Enabling
        } else {
            FigmaMcpStatus::Installed
        }
    }

    pub fn get_template_uuid(&self, installation_uuid: Uuid) -> Option<Uuid> {
        self.locally_installed_servers
            .get(&installation_uuid)
            .map(|server_installation| server_installation.template_uuid())
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn is_server_active(&self, installation_uuid: Uuid) -> bool {
        self.active_servers.contains_key(&installation_uuid)
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn is_server_active_or_pending(&self, uuid: Uuid) -> bool {
        self.is_server_active(uuid) || self.spawned_servers.contains_key(&uuid)
    }

    pub fn get_server_state(&self, installation_uuid: Uuid) -> Option<MCPServerState> {
        self.server_states.get(&installation_uuid).copied()
    }

    pub fn get_server_error_message(&self, installation_uuid: Uuid) -> Option<&str> {
        self.server_error_messages
            .get(&installation_uuid)
            .map(|s| s.as_str())
    }

    pub fn tools_for_server(&self, uuid: Uuid) -> Vec<rmcp::model::Tool> {
        self.active_servers
            .get(&uuid)
            .map(|server| server.tools.clone())
            .unwrap_or_default()
    }

}

#[derive(Debug)]
#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub enum TemplatableMCPServerManagerEvent {
    StateChanged,
    // TODO(aeybel) Right now most of the app doesn't use these events to communicate
    // We should change them so this manager is source of truth and all communication goes through here
    #[allow(dead_code)]
    ServerInstallationAdded(Uuid),
    #[allow(dead_code)]
    ServerInstallationDeleted(Uuid),
    TemplatableMCPServersUpdated,
    LegacyServerConverted,
}

impl Entity for TemplatableMCPServerManager {
    type Event = TemplatableMCPServerManagerEvent;
}

impl SingletonEntity for TemplatableMCPServerManager {}
