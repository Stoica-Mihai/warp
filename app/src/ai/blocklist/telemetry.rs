use serde::Serialize;

/// Coarse approval transition for the plan card's `Use orchestration` toggle.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OrchestrationApprovalStatus {
    Approved,
    Disapproved,
}

/// Stable names for run-wide config fields that can diverge between the
/// dispatched orchestration request and either the original tool call or an
/// active approved config.
pub(crate) mod orchestration_modified_field {
    pub const MODEL_ID: &str = "model_id";
    pub const HARNESS: &str = "harness";
    pub const EXECUTION_MODE: &str = "execution_mode";
    pub const ENVIRONMENT_ID: &str = "environment_id";
    pub const WORKER_HOST: &str = "worker_host";
    pub const AUTH_SECRET: &str = "auth_secret";
}

/// Decision a user took on the run_agents confirmation card.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunAgentsCardDecision {
    Accept,
    AcceptWithoutOrchestration,
    Reject,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PillBarPillKind {
    Orchestrator,
    Child,
}

/// Concrete user actions against an orchestration pill bar entry.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PillBarActionKind {
    /// User clicked the pill body. See `switch_outcome` for what happened next.
    Switch,
    OpenInNewPane,
    OpenInNewTab,
    /// User picked "Focus pane" from a pill's 3-dot menu.
    FocusOpenedConversation,
    Stop,
    Kill,
    TogglePinOn,
    TogglePinOff,
    ViewInOz,
    OpenMenu,
}

/// Outcome of a pill-body click. Closed enum so future navigation outcomes can
/// be added without splitting `Switch` into multiple action variants again.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PillSwitchOutcome {
    /// Pill click navigated within the current pane.
    SwitchedInPlace,
    /// Target conversation was already owned by another visible terminal view;
    /// focus moved there instead of switching in place.
    FocusedExistingPane,
}
