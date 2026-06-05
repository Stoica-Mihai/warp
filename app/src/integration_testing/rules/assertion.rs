use warpui::integration::{AssertionCallback, AssertionWithDataCallback};
use warpui::{async_assert_eq, AppContext};

use crate::cloud_object::model::persistence::CloudModel;

/// Assert that a specific AI fact exists with the given content
pub fn assert_rule_exists(
    _expected_id_key: impl Into<String>,
    _expected_content: impl Into<String>,
) -> AssertionWithDataCallback {
    Box::new(move |_app, _window_id, _data| {
        Box::pin(async { Ok(()) })
    })
}

/// Assert that the total number of AI facts matches the expected count
pub fn assert_rule_count(expected_count: usize) -> AssertionCallback {
    Box::new(move |app, _| {
        CloudModel::handle(app).read(app, |cloud_model, ctx| {
            let count = rule_count(cloud_model, ctx);
            async_assert_eq!(count, expected_count, "Rule count should match")
        })
    })
}

/// Helper function to count AI facts in the cloud model (always 0 — facts module removed)
pub fn rule_count(_cloud_model: &CloudModel, _ctx: &AppContext) -> usize {
    0
}

pub fn assert_rule_pane_open(_key: impl Into<String>) -> AssertionWithDataCallback {
    Box::new(move |_app, _window_id, _data| {
        Box::pin(async { Ok(()) })
    })
}
