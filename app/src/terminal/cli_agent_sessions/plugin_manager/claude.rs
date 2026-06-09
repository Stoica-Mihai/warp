use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{env, fs, io};

use serde_json::Value;

use super::{CliAgentPluginManager, PluginInstructionStep, PluginInstructions};

const PLUGIN_KEY: &str = "warp@claude-code-warp";

// Keep in sync with the plugin version in warpdotdev/claude-code-warp.
// (See the Versioning section of that repo's README.)
const MINIMUM_PLUGIN_VERSION: &str = "2.1.0";

pub(super) struct ClaudeCodePluginManager;

impl ClaudeCodePluginManager {
    pub(super) fn new() -> Self {
        Self
    }
}

impl CliAgentPluginManager for ClaudeCodePluginManager {
    fn minimum_plugin_version(&self) -> &'static str {
        MINIMUM_PLUGIN_VERSION
    }

    fn can_auto_install(&self) -> bool {
        true
    }

    fn is_installed(&self) -> bool {
        let Ok(claude_dir) = claude_home_dir() else {
            return false;
        };
        check_installed(&claude_dir)
    }

    fn install_instructions(&self) -> &'static PluginInstructions {
        &INSTALL_INSTRUCTIONS
    }

    fn update_instructions(&self) -> &'static PluginInstructions {
        &UPDATE_INSTRUCTIONS
    }
}

static INSTALL_INSTRUCTIONS: LazyLock<PluginInstructions> = LazyLock::new(|| {
    PluginInstructions {
        title: "Install Warp Plugin for Claude Code",
        subtitle: "Ensure that jq is installed on your machine. Then, run these commands.",
        steps: &[
            PluginInstructionStep {
                description: "Add the Warp plugin marketplace repository",
                command: "claude plugin marketplace add warpdotdev/claude-code-warp",
                executable: true,
                link: None,
            },
            PluginInstructionStep {
                description: "Install the Warp plugin",
                command: "claude plugin install warp@claude-code-warp",
                executable: true,
                link: None,
            },
        ],
        post_install_notes: &[
            "Restart Claude Code to activate the plugin.",
            "There are some known issues with Claude Code's plugin system. \
             If the plugin is not found after step 1, you can try manually adding an \"extraKnownMarketplaces\" entry to ~/.claude/settings.json.",
        ],
    }
});

static UPDATE_INSTRUCTIONS: LazyLock<PluginInstructions> = LazyLock::new(|| PluginInstructions {
    title: "Update Warp Plugin for Claude Code",
    subtitle: "Run the following commands.",
    steps: &[
        PluginInstructionStep {
            description: "Remove the existing marketplace (if present)",
            command: "claude plugin marketplace remove claude-code-warp",
            executable: true,
            link: None,
        },
        PluginInstructionStep {
            description: "Re-add the marketplace",
            command: "claude plugin marketplace add warpdotdev/claude-code-warp",
            executable: true,
            link: None,
        },
        PluginInstructionStep {
            description: "Install the latest plugin version",
            command: "claude plugin install warp@claude-code-warp",
            executable: true,
            link: None,
        },
    ],
    post_install_notes: &["Restart Claude Code to activate the update."],
});

fn check_installed(claude_dir: &Path) -> bool {
    let plugins_path = claude_dir.join("plugins").join("installed_plugins.json");
    let Ok(contents) = fs::read_to_string(plugins_path) else {
        return false;
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&contents) else {
        return false;
    };
    parsed
        .get("plugins")
        .and_then(|p| p.get(PLUGIN_KEY))
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false)
}

/// Reads the installed version string for the Warp plugin, if present.
fn installed_version(claude_dir: &Path) -> Option<String> {
    let plugins_path = claude_dir.join("plugins").join("installed_plugins.json");
    let contents = fs::read_to_string(plugins_path).ok()?;
    let parsed: Value = serde_json::from_str(&contents).ok()?;
    parsed
        .get("plugins")?
        .get(PLUGIN_KEY)?
        .as_array()?
        .first()?
        .get("version")?
        .as_str()
        .map(|s| s.to_owned())
}

/// Checks `CLAUDE_HOME` env var first, falls back to `~/.claude`.
fn claude_home_dir() -> io::Result<PathBuf> {
    if let Ok(claude_home) = env::var("CLAUDE_HOME") {
        return Ok(PathBuf::from(claude_home));
    }
    dirs::home_dir()
        .map(|home| home.join(".claude"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine home directory",
            )
        })
}

#[cfg(test)]
#[path = "claude_tests.rs"]
mod tests;
