use std::sync::LazyLock;

use super::{CliAgentPluginManager, PluginInstructionStep, PluginInstructions};

// Keep in sync with the plugin version in warpdotdev/gemini-warp.
const MINIMUM_PLUGIN_VERSION: &str = "1.0.0";

pub(super) struct GeminiPluginManager;

impl GeminiPluginManager {
    pub(super) fn new() -> Self {
        Self
    }
}

impl CliAgentPluginManager for GeminiPluginManager {
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

static INSTALL_INSTRUCTIONS: LazyLock<PluginInstructions> = LazyLock::new(|| PluginInstructions {
    title: "Install Warp Plugin for Gemini CLI",
    subtitle: "Run the following command, then restart Gemini CLI.",
    steps: &[PluginInstructionStep {
        description: "Install the Warp extension",
        command:
            "gemini extensions install https://github.com/warpdotdev/gemini-cli-warp --consent",
        executable: true,
        link: None,
    }],
    post_install_notes: &["Restart Gemini CLI to activate the plugin."],
});

static UPDATE_INSTRUCTIONS: LazyLock<PluginInstructions> = LazyLock::new(|| PluginInstructions {
    title: "Update Warp Plugin for Gemini CLI",
    subtitle: "Run the following command, then restart Gemini CLI.",
    steps: &[PluginInstructionStep {
        description: "Update the Warp extension",
        command: "gemini extensions update gemini-warp",
        executable: true,
        link: None,
    }],
    post_install_notes: &["Restart Gemini CLI to activate the update."],
});

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;
