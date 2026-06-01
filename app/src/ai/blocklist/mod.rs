//! This module contains model and view logic for Blocklist AI.
pub(crate) mod action_stubs;
pub(crate) use action_stubs::{
    AIActionStatus, BlocklistAIActionEvent, BlocklistAIActionModel, NewConversationDecision,
    ReadFileContextResult, RunAgentsExecutor, RunAgentsExecutorEvent, RunAgentsSpawningSnapshot,
    ShellCommandExecutor, ShellCommandExecutorEvent, StartAgentExecutor, StartAgentExecutorEvent,
    StartAgentRequest, StartAgentRequestId, coerce_integer_args, compose_run_agents_child_prompt,
    read_local_file_context, run_agents_to_start_agent_mode,
};
pub(crate) mod avatar_disc;
#[path = "block_stubs.rs"]
pub mod block;
pub mod code_block;
mod context_model_stubs;
pub(crate) mod handoff;

pub(crate) mod orchestration_stubs;
pub(crate) use orchestration_stubs::{
    LocalSharedSessionLinkModel, OrchestrationEventService, OrchestrationEventStreamer,
    SendEventResult, TaskStatusSyncModel,
    collect_descendant_conversation_ids_in_spawn_order, conversation_id_for_agent_id,
    descendant_conversation_ids_in_spawn_order, dispatch_focus_or_open_child_agent_pane,
};
pub(crate) mod request_input;
pub(crate) mod response_stream_id;
pub(crate) mod session_context;
pub(crate) use request_input::RequestInput;
pub(crate) use response_stream_id::{ClientIdentifiers, ResponseStreamId};
pub(crate) use session_context::SessionContext;
#[path = "history_model_stubs.rs"]
pub mod history_model;
pub mod inline_action;
mod input_config;
pub(crate) mod keystroke_render;
mod permissions;
mod persistence;
pub mod prompt;
pub mod suggested_agent_mode_workflow_modal;
pub mod suggested_rule_modal;
mod suggestion_chip_view;
pub mod summarization_cancel_dialog;
pub(crate) mod telemetry;
pub mod usage;

pub(crate) mod codebase_index_speedbump_banner;
pub(crate) mod telemetry_banner;
pub(super) mod view_util;

#[cfg(any(test, feature = "integration_tests"))]
pub(crate) use block::model::testing::FakeAIBlockModel;
pub(crate) use block::{init, model, AIBlock, AIBlockEvent, RequestedEditResolution};
pub use block::{keyboard_navigable_buttons, toggleable_items};
pub(crate) use context_model_stubs::{
    block_context_from_terminal_model, AttachmentType, BlocklistAIContextEvent,
    BlocklistAIContextModel, PendingAttachment, PendingFile, PendingQueryState,
};
pub(crate) use history_model::{
    AIQueryHistory, AIQueryHistoryOutputStatus, BlocklistAIHistoryEvent, BlocklistAIHistoryModel,
    ConversationStatusUpdate, FORK_PREFIX, PRE_REWIND_PREFIX,
};
pub use input_config::{InputConfig, InputType, InputTypeAutoDetectionSource};
// BlocklistAIInputModel/Event still needed by some callers; provide minimal stubs inline
pub(crate) use self::input_stubs::{BlocklistAIInputEvent, BlocklistAIInputModel};
mod input_stubs {
    use warpui::{Entity, ModelContext};
    use super::input_config::{InputConfig, InputType, InputTypeAutoDetectionSource};

    #[derive(Debug, Clone)]
    pub enum BlocklistAIInputEvent {
        InputTypeChanged { config: InputConfig },
        LockChanged { config: InputConfig },
    }
    impl BlocklistAIInputEvent {
        pub fn did_update_input_config(&self) -> bool { true }
        pub fn updated_config(&self) -> &InputConfig {
            match self {
                BlocklistAIInputEvent::InputTypeChanged { config } |
                BlocklistAIInputEvent::LockChanged { config } => config,
            }
        }
    }
    pub struct BlocklistAIInputModel {
        pub(super) input_config: InputConfig,
    }
    impl Entity for BlocklistAIInputModel {
        type Event = BlocklistAIInputEvent;
    }
    impl BlocklistAIInputModel {
        pub fn new(_model: std::sync::Arc<parking_lot::FairMutex<crate::terminal::TerminalModel>>,
                   _ctx_model: warpui::ModelHandle<super::context_model_stubs::BlocklistAIContextModel>,
                   _terminal_view_id: warpui::EntityId,
                   _ctx: &mut ModelContext<Self>) -> Self {
            Self { input_config: InputConfig { input_type: InputType::Shell, is_locked: false } }
        }
        pub fn is_ai_input_enabled(&self) -> bool { false }
        pub fn input_config(&self) -> InputConfig { self.input_config }
        pub fn input_type(&self) -> InputType { self.input_config.input_type }
        pub fn is_input_type_locked(&self) -> bool { self.input_config.is_locked }
        pub fn should_run_input_autodetection(&self, _: &warpui::AppContext) -> bool { false }
        pub fn is_autodetection_enabled_for_current_context(&self, _: &warpui::AppContext) -> bool { false }
        pub fn last_ai_autodetection_source(&self) -> Option<InputTypeAutoDetectionSource> { None }
        pub fn last_ai_autodetection_ts(&self) -> Option<instant::Instant> { None }
        pub fn was_lock_set_with_empty_buffer(&self) -> bool { false }
        pub fn set_input_config(&mut self, new_config: InputConfig, _is_empty: bool, _src: Option<InputTypeAutoDetectionSource>, _ctx: &mut ModelContext<Self>) { self.input_config = new_config; }
        pub fn set_input_config_for_classic_mode(&mut self, new_config: InputConfig, _ctx: &mut ModelContext<Self>) { self.input_config = new_config; }
        pub fn set_input_type(&mut self, input_type: InputType, _src: Option<InputTypeAutoDetectionSource>, _ctx: &mut ModelContext<Self>) { self.input_config.input_type = input_type; }
        pub fn enable_autodetection(&mut self, _input_type: InputType, _ctx: &mut ModelContext<Self>) {}
        pub fn handle_input_buffer_submitted(&mut self, _ctx: &mut ModelContext<Self>) { self.input_config = InputConfig { input_type: InputType::Shell, is_locked: false }; }
        pub fn abort_in_progress_detection(&mut self) {}
        pub fn detect_and_set_input_type<C: Send + 'static>(&mut self, _input: crate::terminal::input::decorations::ParsedTokensSnapshot, _: C, _: Option<crate::terminal::model::session::SessionId>, _ctx: &mut ModelContext<Self>) {}
        pub fn unlocked_if_autodetection_enabled(self, _: bool, _: &warpui::AppContext) -> InputConfig { self.input_config }
    }
}
pub use permissions::BlocklistAIPermissions;
#[cfg(test)]
pub use permissions::CommandExecutionPermissionAllowedReason;
#[cfg_attr(target_family = "wasm", allow(unused))]
pub(crate) use persistence::PersistedAIInputType;
pub(crate) use persistence::{PersistedAIInput, SerializedBlockListItem};
pub use suggestion_chip_view::*;
pub use view_util::error_color;
pub(crate) use view_util::{
    ai_brand_color, ai_indicator_height, format_credits,
    get_ai_block_overflow_menu_element_position_id, get_attached_blocks_chip_element_position_id,
    render_ai_agent_mode_icon, render_ai_follow_up_icon, ATTACH_AS_AGENT_MODE_CONTEXT_TEXT,
    CLAUDE_ORANGE, NEW_AGENT_PANE_LABEL,
};

pub use crate::ai::blocklist::block::{secret_redaction, AIBlockResponseRating, TextLocation};
