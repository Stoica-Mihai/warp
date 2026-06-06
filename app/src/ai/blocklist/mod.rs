//! This module contains model and view logic for Blocklist AI.
pub(crate) mod cli_controller;
pub mod code_block;
pub(crate) mod handoff;

pub(crate) mod request_input;
pub(crate) mod response_stream_id;
pub(crate) use request_input::RequestInput;
pub(crate) use response_stream_id::{ResponseStreamId};
pub mod history_model {}
mod input_config;
pub(crate) mod keystroke_render;
pub(crate) mod persistence;
pub mod prompt;
pub(super) mod view_util;

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
pub(crate) use persistence::{PersistedAIInput, SerializedBlockListItem};
pub(crate) use view_util::{
    ai_brand_color, ai_indicator_height,
    render_ai_agent_mode_icon, ATTACH_AS_AGENT_MODE_CONTEXT_TEXT,
    CLAUDE_ORANGE, NEW_AGENT_PANE_LABEL,
};
