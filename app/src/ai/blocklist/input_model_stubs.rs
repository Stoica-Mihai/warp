use std::sync::Arc;

use instant::Instant;
use parking_lot::FairMutex;
use serde::{Deserialize, Serialize};
use session_sharing_protocol::common::{InputMode, InputType as ProtocolInputType};
pub use input_classifier::{InputClassifierDecisionSource, InputType};
use warpui::{AppContext, Entity, EntityId, ModelContext, ModelHandle};

use crate::terminal::model::session::Sessions;
use crate::terminal::TerminalModel;

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

#[derive(Debug, Clone)]
pub enum BlocklistAIInputEvent {
    InputTypeChanged { config: InputConfig },
    LockChanged { config: InputConfig },
}

impl BlocklistAIInputEvent {
    pub fn did_update_input_config(&self) -> bool {
        true
    }

    pub fn updated_config(&self) -> &InputConfig {
        match self {
            BlocklistAIInputEvent::InputTypeChanged { config }
            | BlocklistAIInputEvent::LockChanged { config } => config,
        }
    }
}

pub struct BlocklistAIInputModel {
    input_config: InputConfig,
    was_lock_set_with_empty_buffer: bool,
    last_ai_autodetection_ts: Option<Instant>,
    last_ai_autodetection_source: Option<InputTypeAutoDetectionSource>,
}

impl Entity for BlocklistAIInputModel {
    type Event = BlocklistAIInputEvent;
}

impl BlocklistAIInputModel {
    pub fn new(
        _model: Arc<FairMutex<TerminalModel>>,
        _ai_context_model: warpui::ModelHandle<super::context_model_stubs::BlocklistAIContextModel>,
        _terminal_view_id: EntityId,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            input_config: InputConfig {
                input_type: InputType::Shell,
                is_locked: false,
            },
            was_lock_set_with_empty_buffer: false,
            last_ai_autodetection_ts: None,
            last_ai_autodetection_source: None,
        }
    }

    pub fn input_type(&self) -> InputType {
        self.input_config.input_type
    }

    pub fn is_input_type_locked(&self) -> bool {
        self.input_config.is_locked
    }

    pub fn is_ai_input_enabled(&self) -> bool {
        self.input_config.input_type == InputType::AI
    }

    pub fn input_config(&self) -> InputConfig {
        self.input_config
    }

    pub fn last_ai_autodetection_source(&self) -> Option<InputTypeAutoDetectionSource> {
        self.last_ai_autodetection_source
    }

    pub fn last_ai_autodetection_ts(&self) -> Option<Instant> {
        self.last_ai_autodetection_ts
    }

    pub fn set_input_config_for_classic_mode(
        &mut self,
        new_config: InputConfig,
        ctx: &mut ModelContext<Self>,
    ) {
        self.set_input_config_internal(new_config, None, ctx);
    }

    pub fn set_input_type(
        &mut self,
        input_type: InputType,
        decision_source: Option<InputTypeAutoDetectionSource>,
        ctx: &mut ModelContext<Self>,
    ) {
        let current_config = self.input_config();
        self.set_input_config_internal(current_config.with_input_type(input_type), decision_source, ctx);
    }

    fn set_input_config_internal(
        &mut self,
        new_config: InputConfig,
        decision_source: Option<InputTypeAutoDetectionSource>,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        if self.input_config == new_config {
            self.last_ai_autodetection_source = decision_source;
            return false;
        }
        let old_config = self.input_config;
        self.input_config = new_config;
        self.last_ai_autodetection_source = decision_source;
        if old_config.input_type != new_config.input_type {
            ctx.emit(BlocklistAIInputEvent::InputTypeChanged { config: new_config });
        }
        if old_config.is_locked != new_config.is_locked {
            ctx.emit(BlocklistAIInputEvent::LockChanged { config: new_config });
        }
        true
    }

    pub fn set_input_config(
        &mut self,
        new_config: InputConfig,
        is_input_buffer_empty: bool,
        decision_source: Option<InputTypeAutoDetectionSource>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.set_input_config_internal(new_config, decision_source, ctx);
        self.was_lock_set_with_empty_buffer = new_config.is_locked && is_input_buffer_empty;
    }

    pub fn should_run_input_autodetection(&self, _app: &AppContext) -> bool {
        false
    }

    pub fn is_autodetection_enabled_for_current_context(&self, _app: &AppContext) -> bool {
        false
    }

    pub fn enable_autodetection(&mut self, input_type: InputType, ctx: &mut ModelContext<Self>) {
        self.set_input_config_internal(
            InputConfig {
                input_type,
                is_locked: false,
            },
            None,
            ctx,
        );
    }

    pub fn handle_input_buffer_submitted(&mut self, ctx: &mut ModelContext<Self>) {
        self.set_input_config(
            InputConfig {
                input_type: InputType::Shell,
                is_locked: false,
            },
            true,
            None,
            ctx,
        );
    }

    pub fn was_lock_set_with_empty_buffer(&self) -> bool {
        self.was_lock_set_with_empty_buffer
    }

    pub fn abort_in_progress_detection(&mut self) {}

    pub fn detect_and_set_input_type<C: Send + 'static>(
        &mut self,
        _input: crate::terminal::input::decorations::ParsedTokensSnapshot,
        _completion_context: C,
        _session_id: Option<crate::terminal::model::session::SessionId>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }
}
