//! Ambient agent task types and utilities.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use warp_cli::agent::Harness;
use warpui::{SingletonEntity, View, ViewContext};

use super::AmbientAgentTaskId;
use crate::server::server_api::ServerApiProvider;
use crate::view_components::DismissibleToast;
use crate::workspace::ToastStack;

/// Runtime configuration snapshot for agent execution.
///
/// This is the merged/resolved config used when spawning or running an agent.
/// It combines settings from config files and CLI args.
/// Unlike `AgentConfig` (the cloud model), field names here use the runtime format
/// (e.g. `model_id` instead of `base_model_id`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AgentConfigSnapshot {
    /// Config name for searchability/traceability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_prompt: Option<String>,
    /// MCP server configuration map (unwrapped; no `mcpServers` wrapper).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Map<String, serde_json::Value>>,
    /// Profile ID for local agent runs. This configures the terminal session
    /// with the specified execution profile. Only used for local runs, not cloud runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    /// Self-hosted worker ID that should execute this task.
    /// If None or Some("warp"), the task will be dispatched to Warp-hosted (Namespace) workers.
    /// Otherwise, the task will only be assigned to a connected self-hosted worker with matching ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_host: Option<String>,
    /// Skill spec to use as the base prompt for the agent.
    /// Format: "skill_name", "repo:skill_name", or "org/repo:skill_name".
    /// The skill is resolved at runtime in the agent environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_spec: Option<String>,
    /// Whether computer use is enabled for this agent run.
    /// If None, the default behavior is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_use_enabled: Option<bool>,
    /// Execution harness for the agent run.
    /// If None, we use Warp's default ("oz").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<HarnessConfig>,
    /// Authentication secrets for third-party harnesses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_auth_secrets: Option<HarnessAuthSecretsConfig>,
}

/// Configuration for a third-party execution harness.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HarnessConfig {
    /// The harness type, e.g. [`Harness::Claude`].
    #[serde(
        rename = "type",
        serialize_with = "serialize_harness",
        deserialize_with = "deserialize_harness"
    )]
    pub harness_type: Harness,
    /// The model to use with this harness. None means use the harness default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Optional reasoning level for harnesses that support it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_level: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessModelConfig {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_level: Option<String>,
}

impl HarnessConfig {
    /// Builds a harness config from just the harness type.
    pub fn from_harness_type(harness_type: Harness) -> Self {
        Self {
            harness_type,
            model_id: None,
            reasoning_level: None,
        }
    }

    pub fn model_config(&self) -> Option<HarnessModelConfig> {
        self.model_id
            .as_ref()
            .filter(|id| !id.is_empty())
            .map(|model_id| HarnessModelConfig {
                model_id: model_id.clone(),
                reasoning_level: self.reasoning_level.clone(),
            })
    }
}


fn serialize_harness<S: Serializer>(harness: &Harness, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(harness.config_name())
}

fn deserialize_harness<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Harness, D::Error> {
    let name = String::deserialize(deserializer)?;
    Ok(Harness::from_config_name(&name).unwrap_or_else(|| {
        log::warn!("Unknown harness config name: {name:?}; treating as Unknown");
        Harness::Unknown
    }))
}

/// Authentication secrets for third-party harnesses.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HarnessAuthSecretsConfig {
    /// Name of a managed secret for Claude Code harness authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_auth_secret_name: Option<String>,
    /// Name of a managed secret for Codex harness authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_auth_secret_name: Option<String>,
}

impl AgentConfigSnapshot {
    /// Returns true if this config is empty (no options are set).
    pub fn is_empty(&self) -> bool {
        let Self {
            name,
            environment_id,
            model_id,
            base_prompt,
            mcp_servers,
            profile_id,
            worker_host,
            skill_spec,
            computer_use_enabled,
            harness,
            harness_auth_secrets,
        } = self;

        name.is_none()
            && environment_id.is_none()
            && model_id.is_none()
            && base_prompt.is_none()
            && mcp_servers.is_none()
            && profile_id.is_none()
            && worker_host.is_none()
            && skill_spec.is_none()
            && computer_use_enabled.is_none()
            && harness.is_none()
            && harness_auth_secrets.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSource {
    Linear,
    AgentWebhook,
    Slack,
    Cli,
    ScheduledAgent,
    Interactive,
    WebApp,
    GitHubAction,
    CloudMode,
}

impl AgentSource {
    pub fn as_str(&self) -> &str {
        match self {
            AgentSource::Linear => "LINEAR",
            AgentSource::AgentWebhook => "API",
            AgentSource::Slack => "SLACK",
            AgentSource::Cli => "CLI",
            AgentSource::ScheduledAgent => "SCHEDULED_AGENT",
            // The public API's run source for local interactive tasks is named
            // `LOCAL`.
            AgentSource::Interactive => "LOCAL",
            AgentSource::WebApp => "WEB_APP",
            AgentSource::GitHubAction => "GITHUB_ACTION",
            AgentSource::CloudMode => "CLOUD_MODE",
        }
    }

}


/// Cancel an ambient agent task and show a toast with the result.
pub fn cancel_task_with_toast<V: View>(task_id: AmbientAgentTaskId, ctx: &mut ViewContext<V>) {
    let ai_client = ServerApiProvider::handle(ctx).as_ref(ctx).get_ai_client();
    let window_id = ctx.window_id();
    ctx.spawn(
        async move { ai_client.cancel_ambient_agent_task(&task_id).await },
        move |_view, result, ctx| {
            let message = match result {
                Ok(()) => "Task cancelled".to_string(),
                Err(e) => {
                    log::error!("Failed to cancel task: {e}");
                    format!("Failed to cancel task: {e}")
                }
            };
            ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
                let toast = DismissibleToast::default(message);
                toast_stack.add_ephemeral_toast(toast, window_id, ctx);
            });
        },
    );
}

/// Cancel an ambient agent task without surfacing a toast to the user.
pub fn cancel_task_silently<V: View>(task_id: AmbientAgentTaskId, ctx: &mut ViewContext<V>) {
    let ai_client = ServerApiProvider::handle(ctx).as_ref(ctx).get_ai_client();
    ctx.spawn(
        async move { ai_client.cancel_ambient_agent_task(&task_id).await },
        move |_view, result, _| {
            if let Err(e) = result {
                log::error!("Failed to cancel task: {e}");
            }
        },
    );
}

