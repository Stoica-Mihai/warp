//! Tests for [`inherit_share_for_local_child`]. These verify the pure
//! branching independent of the PaneGroup dispatch code. The behavior
//! is gated by `FeatureFlag::OrchestrationViewerPillBar` so each case
//! must override it explicitly.

use uuid::Uuid;

use super::*;

fn new_task_id() -> AmbientAgentTaskId {
    Uuid::new_v4().to_string().parse().unwrap()
}

fn user_source(task_id: Option<&str>) -> SharedSessionSource {
    SharedSessionSource::user(task_id.map(str::to_owned))
}

fn ambient_source(task_id: Option<&str>) -> SharedSessionSource {
    SharedSessionSource::ambient_agent(task_id.map(str::to_owned))
}

#[test]
fn inherit_share_returns_no_when_feature_flag_disabled() {
    let _guard = FeatureFlag::OrchestrationViewerPillBar.override_enabled(false);
    let host = user_source(Some("host-task"));
    let result = inherit_share_for_local_child(Some(&host), new_task_id());
    assert!(matches!(result, IsSharedSessionCreator::No));
}

#[test]
fn inherit_share_returns_no_when_host_is_not_sharing() {
    let _guard = FeatureFlag::OrchestrationViewerPillBar.override_enabled(true);
    let result = inherit_share_for_local_child(None, new_task_id());
    assert!(matches!(result, IsSharedSessionCreator::No));
}

#[test]
fn inherit_share_returns_no_when_host_user_share_has_no_task_id() {
    let _guard = FeatureFlag::OrchestrationViewerPillBar.override_enabled(true);
    let host = user_source(None);
    let result = inherit_share_for_local_child(Some(&host), new_task_id());
    assert!(
        matches!(result, IsSharedSessionCreator::No),
        "hosts without a stamped task_id must NOT cascade; the viewer cannot enumerate \
         children via REST without a task_id"
    );
}

#[test]
fn inherit_share_returns_no_when_host_ambient_share_has_no_task_id() {
    let _guard = FeatureFlag::OrchestrationViewerPillBar.override_enabled(true);
    let host = ambient_source(None);
    let result = inherit_share_for_local_child(Some(&host), new_task_id());
    assert!(matches!(result, IsSharedSessionCreator::No));
}
