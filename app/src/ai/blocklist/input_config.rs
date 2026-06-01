use serde::{Deserialize, Serialize};
use session_sharing_protocol::common::{InputMode, InputType as ProtocolInputType};
pub use input_classifier::{InputClassifierDecisionSource, InputType};
use warpui::AppContext;

/// The source of the final input type decision applied to the user input.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputTypeAutoDetectionSource {
    InputClassifierDecisionSource(InputClassifierDecisionSource),
    ManualToggle,
    ShellPrefix,
    AttachmentForcedAi,
    Denylist,
    HistoryMatch,
    NaturalLanguageAgentFollowUpAllowList,
    HistorySelection,
    WorkflowInsertion,
    CommandAutosuggestionAccepted,
    AgentQueryAutosuggestionAccepted,
    ConversationContextRender,
    ContinueConversation,
    OnboardingAgentPrompt,
    StartNewConversation,
    AskAi,
    SlashCommand,
    InlineAgentViewEntry,
    CloudHandoffEnter,
    CloudHandoffExit,
    AgentModePrefix,
    InlineCodeReviewSend,
    SessionSharingApply,
    FullscreenInlineHistoryCycling,
    RestoreSavedConfig,
    ClassicModeReset,
    VoiceInputToggle,
    AtContextMenuInsert,
}

impl From<InputClassifierDecisionSource> for InputTypeAutoDetectionSource {
    fn from(value: InputClassifierDecisionSource) -> Self {
        Self::InputClassifierDecisionSource(value)
    }
}

/// Configuration for the terminal pane's input.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfig {
    pub input_type: InputType,
    pub is_locked: bool,
}

impl InputConfig {
    pub fn new(_app: &AppContext) -> Self {
        InputConfig {
            input_type: InputType::Shell,
            is_locked: false,
        }
    }

    pub fn with_toggled_type(self) -> Self {
        let input_type = if self.input_type.is_ai() {
            InputType::Shell
        } else {
            InputType::AI
        };
        Self { input_type, ..self }
    }

    pub fn with_shell_type(self) -> Self {
        Self {
            input_type: InputType::Shell,
            ..self
        }
    }

    pub fn with_input_type(self, input_type: InputType) -> Self {
        Self { input_type, ..self }
    }

    pub fn unlocked_if_autodetection_enabled(self, _is_in_fullscreen_agent_view: bool, _app: &AppContext) -> Self {
        Self {
            is_locked: false,
            ..self
        }
    }

    pub fn locked(self) -> Self {
        Self {
            is_locked: true,
            ..self
        }
    }

    pub fn is_ai(&self) -> bool {
        self.input_type == InputType::AI
    }

    pub fn is_shell(&self) -> bool {
        self.input_type == InputType::Shell
    }
}

impl From<InputConfig> for InputMode {
    fn from(config: InputConfig) -> Self {
        let protocol_input_type = match config.input_type {
            InputType::Shell => ProtocolInputType::Shell,
            InputType::AI => ProtocolInputType::AI,
        };
        InputMode::new(protocol_input_type, config.is_locked)
    }
}
