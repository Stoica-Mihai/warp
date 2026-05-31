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
pub mod block;
pub mod code_block;
mod context_model;
pub(crate) mod handoff;

pub(crate) mod local_shared_session_link_model;
pub(crate) mod orchestration_conversation_links;
pub(crate) mod orchestration_event_streamer;
pub(crate) mod orchestration_events;
pub(crate) mod orchestration_topology;
pub(crate) mod request_input;
pub(crate) mod response_stream_id;
pub(crate) mod session_context;
pub(crate) mod task_status_sync_model;
pub(crate) use request_input::RequestInput;
pub(crate) use response_stream_id::{ClientIdentifiers, ResponseStreamId};
pub(crate) use session_context::SessionContext;
pub mod history_model;
pub mod inline_action;
mod input_model;
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
pub(crate) use context_model::{
    block_context_from_terminal_model, AttachmentType, BlocklistAIContextEvent,
    BlocklistAIContextModel, PendingAttachment, PendingFile, PendingQueryState,
};
pub(crate) use history_model::{
    AIQueryHistory, AIQueryHistoryOutputStatus, BlocklistAIHistoryEvent, BlocklistAIHistoryModel,
    ConversationStatusUpdate, FORK_PREFIX, PRE_REWIND_PREFIX,
};
pub(crate) use input_model::{
    BlocklistAIInputEvent, BlocklistAIInputModel, InputConfig, InputType,
    InputTypeAutoDetectionSource,
};
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
