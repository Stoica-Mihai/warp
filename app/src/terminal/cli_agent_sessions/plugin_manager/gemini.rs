use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{fs, io};

use serde_json::Value;

use super::{compare_versions, CliAgentPluginManager, PluginInstructionStep, PluginInstructions};

const EXTENSION_NAME: &str = "gemini-warp";

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

    fn can_auto_install(&self) -> bool {
        true
    }

    fn is_installed(&self) -> bool {
        let Ok(extensions_dir) = gemini_extensions_dir() else {
            return false;
        };
        check_installed(&extensions_dir)
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

fn check_installed(extensions_dir: &Path) -> bool {
    let manifest_path = extensions_dir
        .join(EXTENSION_NAME)
        .join("gemini-extension.json");
    let Ok(contents) = fs::read_to_string(manifest_path) else {
        return false;
    };
    serde_json::from_str::<Value>(&contents).is_ok()
}

/// Reads the installed version string for the Warp extension, if present.
fn installed_version(extensions_dir: &Path) -> Option<String> {
    let manifest_path = extensions_dir
        .join(EXTENSION_NAME)
        .join("gemini-extension.json");
    let contents = fs::read_to_string(manifest_path).ok()?;
    let parsed: Value = serde_json::from_str(&contents).ok()?;
    parsed.get("version")?.as_str().map(|s| s.to_owned())
}

/// Returns the path to `~/.gemini/extensions`.
fn gemini_extensions_dir() -> io::Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".gemini").join("extensions"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine home directory",
            )
        })
}

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;
