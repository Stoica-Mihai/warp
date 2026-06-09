pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod gemini;
pub(crate) mod opencode;

use std::cmp::Ordering;

use claude::ClaudeCodePluginManager;
use codex::CodexPluginManager;
use gemini::GeminiPluginManager;
use opencode::OpenCodePluginManager;

use crate::features::FeatureFlag;
use crate::terminal::CLIAgent;

/// Distinguishes whether the plugin instructions modal should show install or update steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginModalKind {
    Install,
    Update,
}

/// A single step in the plugin install/update instructions pane.
pub(crate) struct PluginInstructionStep {
    pub description: &'static str,
    pub command: &'static str,
    /// When true, the code block shows a "Run" button that inserts the command into the terminal.
    /// Defaults-by-convention to `true`; set to `false` for steps that are not runnable
    /// (e.g. config file snippets).
    pub executable: bool,
    /// Optional URL rendered as a clickable "Learn more" link after the description.
    /// When set with an empty `command`, the code block is omitted entirely.
    pub link: Option<&'static str>,
}

/// All content needed to render the plugin instructions pane for a given agent.
pub(crate) struct PluginInstructions {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub steps: &'static [PluginInstructionStep],
    /// Displayed after the steps in the same style as the subtitle, one per paragraph.
    pub post_install_notes: &'static [&'static str],
}

/// Compares two `X.Y.Z` version strings.
/// Returns `Ordering::Less` if `a < b`, etc.
/// Unparseable components are treated as 0.
pub(crate) fn compare_versions(a: &str, b: &str) -> Ordering {
    let parse = |s: &str| -> [u64; 3] {
        let mut parts = s.splitn(3, '.');
        let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        [major, minor, patch]
    };
    parse(a).cmp(&parse(b))
}

/// Manages the Warp notification plugin for a specific CLI agent.
///
/// Each supported CLI agent has its own implementation that knows how to
/// check installation state and perform install/update operations.
pub(crate) trait CliAgentPluginManager: Send + Sync {
    fn minimum_plugin_version(&self) -> &'static str;
    fn can_auto_install(&self) -> bool;

    fn is_installed(&self) -> bool {
        false
    }

    fn install_instructions(&self) -> &'static PluginInstructions;

    fn supports_update(&self) -> bool {
        true
    }

    fn update_instructions(&self) -> &'static PluginInstructions;
}

pub(crate) fn plugin_manager_for(agent: CLIAgent) -> Option<Box<dyn CliAgentPluginManager>> {
    match agent {
        CLIAgent::Claude => Some(Box::new(ClaudeCodePluginManager::new())),
        CLIAgent::OpenCode
            if FeatureFlag::OpenCodeNotifications.is_enabled()
                && FeatureFlag::HOANotifications.is_enabled() =>
        {
            Some(Box::new(OpenCodePluginManager))
        }
        CLIAgent::Codex
            if FeatureFlag::CodexNotifications.is_enabled()
                && FeatureFlag::HOANotifications.is_enabled() =>
        {
            Some(Box::new(CodexPluginManager))
        }
        CLIAgent::Gemini
            if FeatureFlag::GeminiNotifications.is_enabled()
                && FeatureFlag::HOANotifications.is_enabled() =>
        {
            Some(Box::new(GeminiPluginManager::new()))
        }
        CLIAgent::OpenCode
        | CLIAgent::Codex
        | CLIAgent::Gemini
        | CLIAgent::Amp
        | CLIAgent::Droid
        | CLIAgent::Copilot
        | CLIAgent::Pi
        | CLIAgent::Auggie
        | CLIAgent::CursorCli
        | CLIAgent::Hermes
        | CLIAgent::Goose
        | CLIAgent::Vibe
        | CLIAgent::Unknown => None,
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
