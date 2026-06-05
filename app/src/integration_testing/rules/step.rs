use warpui::integration::TestStep;

/// Create a personal rule (no-op — facts module removed)
pub fn create_a_personal_rule(
    _key: impl Into<String>,
    _name: impl Into<String>,
    _content: impl Into<String>,
) -> TestStep {
    TestStep::new("Create a personal rule (no-op)")
}

/// Open the rule pane (no-op — facts module removed)
pub fn open_rule_pane(_window_key: impl Into<String>, _key: impl Into<String>) -> TestStep {
    TestStep::new("Open rule pane (no-op)")
}

/// Update a rule's content (no-op — facts module removed)
pub fn update_rule_content(
    _fact_key: impl Into<String>,
    _new_content: impl Into<String>,
) -> TestStep {
    TestStep::new("Update rule content (no-op)")
}
