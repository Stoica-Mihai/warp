use super::{CliAgentPluginManager, GeminiPluginManager};

#[test]
fn minimum_version() {
    assert_eq!(
        GeminiPluginManager::new().minimum_plugin_version(),
        "1.0.0"
    );
}

#[test]
fn install_instructions_has_steps() {
    let instructions = GeminiPluginManager::new().install_instructions();
    assert!(!instructions.steps.is_empty());
    assert!(!instructions.title.is_empty());
}

#[test]
fn update_instructions_has_steps() {
    let instructions = GeminiPluginManager::new().update_instructions();
    assert!(!instructions.steps.is_empty());
    assert!(!instructions.title.is_empty());
}
