use serde::Serialize;

/// The entry point through which a local-to-cloud handoff was initiated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffEntryPoint {
    /// User typed `&` in the input to enter handoff compose mode.
    #[default]
    Ampersand,
    /// User used the `/handoff` slash command.
    SlashCommand,
    /// User clicked the "Hand off to cloud" chip in the footer toolbar.
    FooterChip,
    /// The client automatically initiated handoff for an eligible local agent.
    Automatic,
}
