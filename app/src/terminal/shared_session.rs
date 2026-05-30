//! Vestigial session-sharing leftovers.
//!
//! Warp's terminal session-sharing (cloud feature) has been stripped. A few
//! plain data types survive for sites that haven't been fully cleaned up yet.

use session_sharing_protocol::common::SessionId;
use session_sharing_protocol::sharer::SessionSourceType;

use crate::channel::ChannelState;

#[derive(Debug, Clone, Default)]
pub enum IsSharedSessionCreator {
    #[default]
    No,
}

#[derive(Debug, Clone)]
pub struct SharedSessionSource {
    pub source_type: SessionSourceType,
    pub source_task_id: Option<String>,
}

impl SharedSessionSource {
    pub fn user(source_task_id: Option<String>) -> Self {
        Self { source_type: SessionSourceType::User, source_task_id }
    }
    pub fn ambient_agent(task_id: Option<String>) -> Self {
        Self {
            source_type: SessionSourceType::AmbientAgent { task_id: task_id.clone() },
            source_task_id: task_id,
        }
    }
    pub fn orchestrator_task_id(&self) -> Option<&str> {
        self.source_task_id.as_deref().or(match &self.source_type {
            SessionSourceType::AmbientAgent { task_id } => task_id.as_deref(),
            SessionSourceType::User => None,
        })
    }
}

impl Default for SharedSessionSource {
    fn default() -> Self { Self::user(None) }
}

pub fn join_native_intent(session_id: &SessionId) -> String {
    format!("{}://shared_session/{}", ChannelState::url_scheme(), session_id)
}

pub fn join_link(session_id: &SessionId) -> String {
    let use_web_url = !ChannelState::uses_staging_server() || cfg!(feature = "release_bundle");
    if use_web_url {
        format!("{}/session/{}", ChannelState::server_root_url(), session_id)
    } else {
        join_native_intent(session_id)
    }
}

use crate::terminal::model::terminal_model::BlockIndex;

/// Scrollback selection for a (now-removed) shared session. Retained as a
/// plain data enum for the surviving construction sites.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedSessionScrollbackType {
    None,
    FromBlock { block_index: BlockIndex },
    All,
}
