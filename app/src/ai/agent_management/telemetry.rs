use serde::Serialize;

/// Which setup-guide workflow step the user interacted with.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupGuideStep {
    /// Step 1: Create environment (slash command)
    CreateEnvironment,
    /// Step 1: Create environment (CLI command)
    CreateEnvironmentCli,
    /// Step 2: Create Slack integration
    CreateSlackIntegration,
    /// Step 2: Create Linear integration
    CreateLinearIntegration,
}
