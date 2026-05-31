use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use shell_words::quote as shell_quote;
use uuid::Uuid;
use warp_cli::agent::Harness;

use crate::ai::ambient_agents::task::{
    normalize_orchestrator_agent_name, HarnessConfig, HarnessModelConfig,
};
use crate::ai::ambient_agents::{AgentConfigSnapshot, AmbientAgentTaskId};
use crate::ai::local_child_harnesses::local_child_harness_disabled_message;
use crate::server::server_api::ai::AIClient;
use crate::terminal::shell::ShellType;

#[derive(Clone)]
pub(super) struct PreparedLocalHarnessLaunch {
    pub command: String,
    pub env_vars: HashMap<OsString, OsString>,
    pub run_id: String,
    pub task_id: AmbientAgentTaskId,
}

pub(super) fn normalize_local_child_harness(harness_type: &str) -> Option<Harness> {
    Harness::parse_local_child_harness(harness_type)
}

pub(super) fn validate_local_harness_shell(shell_type: Option<ShellType>) -> Result<(), String> {
    match shell_type {
        Some(ShellType::Bash) | Some(ShellType::Zsh) | Some(ShellType::Fish) => Ok(()),
        Some(ShellType::PowerShell) => Err(
            "Local child harnesses currently require bash, zsh, or fish; PowerShell is not supported."
                .to_string(),
        ),
        None => Err(
            "Local child harnesses currently require a detected bash, zsh, or fish session."
                .to_string(),
        ),
    }
}

pub(super) fn build_local_claude_child_command(prompt: &str) -> String {
    let session_id = Uuid::new_v4();
    let quoted_prompt = shell_quote(prompt);
    format!("claude --session-id {session_id} --dangerously-skip-permissions {quoted_prompt}")
}

pub(super) fn build_local_opencode_child_command(prompt: &str) -> String {
    let quoted_prompt = shell_quote(prompt);
    format!("opencode --prompt {quoted_prompt}")
}

pub(super) fn build_local_codex_child_command(prompt: &str) -> String {
    let quoted_prompt = shell_quote(prompt);
    format!("codex --dangerously-bypass-approvals-and-sandbox {quoted_prompt}")
}

pub(super) fn local_child_task_config(
    harness: Harness,
    agent_name: Option<String>,
) -> Option<AgentConfigSnapshot> {
    let agent_name = agent_name
        .as_deref()
        .and_then(normalize_orchestrator_agent_name);
    match harness {
        Harness::Oz | Harness::Unknown => None,
        Harness::Claude | Harness::OpenCode | Harness::Gemini | Harness::Codex => {
            Some(AgentConfigSnapshot {
                name: agent_name,
                harness: Some(HarnessConfig::from_harness_type(harness)),
                ..Default::default()
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_local_harness_child_launch(
    prompt: String,
    harness_type: String,
    model_id: Option<String>,
    parent_run_id: Option<String>,
    agent_name: Option<String>,
    shell_type: Option<ShellType>,
    _startup_directory: Option<PathBuf>,
    ai_client: Arc<dyn AIClient>,
) -> Result<PreparedLocalHarnessLaunch, String> {
    let harness_model_config =
        model_id
            .filter(|id| !id.is_empty())
            .map(|model_id| HarnessModelConfig {
                model_id,
                reasoning_level: None,
            });
    let Some(harness) = normalize_local_child_harness(&harness_type) else {
        let harness_name = harness_type.trim();
        return Err(if harness_name.is_empty() {
            "Local child harness type is missing.".to_string()
        } else {
            format!("Unsupported local child harness '{harness_name}'.")
        });
    };
    if let Some(message) = local_child_harness_disabled_message(harness) {
        return Err(message.to_string());
    }
    validate_local_harness_shell(shell_type)?;
    let command = match harness {
        Harness::Oz => unreachable!("normalize_local_child_harness filters out Oz"),
        Harness::Unknown => unreachable!("normalize_local_child_harness filters out Unknown"),
        Harness::Claude => build_local_claude_child_command(&prompt),
        Harness::Codex => build_local_codex_child_command(&prompt),
        Harness::OpenCode => build_local_opencode_child_command(&prompt),
        Harness::Gemini => unreachable!("normalize_local_child_harness filters out Gemini"),
    };

    let task_id = ai_client
        .create_agent_task(
            prompt.clone(),
            None,
            parent_run_id.clone(),
            local_child_task_config(harness, agent_name),
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to create local {} child task: {error}",
                harness.display_name()
            )
        })?;

    let mut env_vars: HashMap<OsString, OsString> = HashMap::new();
    // Propagate the selected model to Claude Code via ANTHROPIC_MODEL.
    if harness == Harness::Claude {
        if let Some(ref cfg) = harness_model_config {
            env_vars.insert(
                OsString::from("ANTHROPIC_MODEL"),
                OsString::from(&cfg.model_id),
            );
        }
    }

    Ok(PreparedLocalHarnessLaunch {
        command,
        env_vars,
        run_id: task_id.to_string(),
        task_id,
    })
}

#[cfg(test)]
#[path = "local_harness_launch_tests.rs"]
mod tests;
