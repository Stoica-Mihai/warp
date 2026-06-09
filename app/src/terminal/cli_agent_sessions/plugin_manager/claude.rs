use std::sync::LazyLock;

use super::{CliAgentPluginManager, PluginInstructionStep, PluginInstructions};

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

