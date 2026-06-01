//! Stubs replacing deleted ai/blocklist/block.rs + block/ directory.
//! Binary shrink comes when callers are cleaned up in Phase E; stubs keep the
//! tree green in the interim.

use parking_lot::FairMutex;
use std::sync::Arc;

use warpui::elements::Empty;
use warpui::{
    AppContext, Element, Entity, EntityId, ModelContext, ModelHandle, TypedActionView, View,
    ViewContext, ViewHandle,
};

// ─── Re-exports ──────────────────────────────────────────────────────────────

pub use crate::util::text_location::TextLocation;

// ─── secret_redaction sub-module ─────────────────────────────────────────────

pub mod secret_redaction {
    pub use crate::secret_redaction::{
        find_secrets_in_text, find_secrets_in_text_with_levels,
        find_secrets_in_text_with_levels_using_regex, SECRET_REDACTION_REPLACEMENT_CHARACTER,
    };

    #[derive(Default, Debug)]
    pub struct SecretRedactionState;
}

// ─── keyboard_navigable_buttons / toggleable_items ───────────────────────────

pub mod keyboard_navigable_buttons {
    pub use crate::terminal::view::keyboard_navigable_buttons::*;
}

pub mod toggleable_items {
    pub use crate::terminal::view::toggleable_items::*;
}

// ─── status_bar ──────────────────────────────────────────────────────────────

pub mod status_bar {
    use parking_lot::FairMutex;
    use std::sync::Arc;

    use warpui::elements::Empty;
    use warpui::{AppContext, Element, Entity, EntityId, ModelHandle, TypedActionView, View, ViewContext, ViewHandle};

    use crate::ai::blocklist::BlocklistAIInputModel;
    use super::cli_controller::CLISubagentController;
    use crate::ai::blocklist::summarization_cancel_dialog::SummarizationCancelDialog;
    use crate::terminal::input::buffer_model::InputBufferModel;
    use crate::terminal::model_events::ModelEventDispatcher;
    use crate::terminal::view::ambient_agent::AmbientAgentViewModel;
    use crate::terminal::TerminalModel;

    pub fn init(_app: &mut AppContext) {}

    pub enum BlocklistAIStatusBarEvent {
        SummarizationCancelDialogToggled { is_open: bool },
    }

    #[derive(Debug, Clone)]
    pub enum BlocklistAIStatusBarAction {}

    pub struct BlocklistAIStatusBar {
        summarization_cancel_dialog: ViewHandle<SummarizationCancelDialog>,
    }

    impl BlocklistAIStatusBar {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            _cli_subagent_controller: ModelHandle<CLISubagentController>,
            _input_model: ModelHandle<BlocklistAIInputModel>,
            _input_buffer_model: ModelHandle<InputBufferModel>,
            _model_event_dispatcher: &ModelHandle<ModelEventDispatcher>,
            _terminal_model: Arc<FairMutex<TerminalModel>>,
            _ambient_agent_view_model: Option<ModelHandle<AmbientAgentViewModel>>,
            _terminal_view_id: EntityId,
            ctx: &mut ViewContext<Self>,
        ) -> Self {
            Self {
                summarization_cancel_dialog: ctx
                    .add_typed_action_view(|_| SummarizationCancelDialog::default()),
            }
        }

        pub fn should_show_summarization_cancel_dialog(&self, _app: &AppContext) -> bool {
            false
        }

        pub fn summarization_cancel_dialog_handle(&self) -> &ViewHandle<SummarizationCancelDialog> {
            &self.summarization_cancel_dialog
        }

        pub fn handle_ctrl_c(&mut self, _ctx: &mut ViewContext<Self>) {}
    }

    impl Entity for BlocklistAIStatusBar {
        type Event = BlocklistAIStatusBarEvent;
    }

    impl View for BlocklistAIStatusBar {
        fn ui_name() -> &'static str {
            "BlocklistAIStatusBar"
        }

        fn render(&self, _app: &AppContext) -> Box<dyn Element> {
            Empty::new().finish()
        }
    }

    impl TypedActionView for BlocklistAIStatusBar {
        type Action = BlocklistAIStatusBarAction;

        fn handle_action(&mut self, _: &BlocklistAIStatusBarAction, _: &mut ViewContext<Self>) {}
    }
}

// ─── cli ──────────────────────────────────────────────────────────────────────

pub mod cli {
    use warpui::elements::Empty;
    use warpui::{AppContext, Element, Entity, View};

    pub enum CLISubagentViewEvent {}

    pub struct CLISubagentView;

    impl CLISubagentView {
        pub fn clear_all_selections(&mut self, _app: &AppContext) {}
        pub fn selected_text(&self, _app: &AppContext) -> Option<String> { None }
    }

    impl Entity for CLISubagentView {
        type Event = CLISubagentViewEvent;
    }

    impl View for CLISubagentView {
        fn ui_name() -> &'static str {
            "CLISubagentView"
        }

        fn render(&self, _app: &AppContext) -> Box<dyn Element> {
            Empty::new().finish()
        }
    }
}

// ─── cli_controller ───────────────────────────────────────────────────────────

pub mod cli_controller {
    use parking_lot::FairMutex;
    use std::sync::Arc;

    use serde::{Deserialize, Serialize};
    use warpui::{Entity, EntityId, ModelContext, ModelHandle};

    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::agent::task::TaskId;
    use crate::ai::agent::AIAgentActionId;
        use crate::terminal::model::block::BlockId;
    use crate::terminal::model_events::ModelEventDispatcher;
    use crate::terminal::TerminalModel;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    pub enum UserTakeOverReason {
        Manual,
        Stop,
        TransferFromAgent { reason: String },
    }

    impl UserTakeOverReason {
        pub fn is_stop(&self) -> bool {
            matches!(self, Self::Stop)
        }

        pub fn is_transfer_from_agent(&self) -> bool {
            matches!(self, Self::TransferFromAgent { .. })
        }

        pub fn transfer_reason(&self) -> Option<&str> {
            match self {
                Self::TransferFromAgent { reason } => Some(reason.as_str()),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub enum LongRunningCommandControlState {
        Agent { is_blocked: bool, should_hide_responses: bool },
        User { reason: UserTakeOverReason },
    }

    impl LongRunningCommandControlState {
        pub fn is_agent_in_control(&self) -> bool {
            matches!(self, Self::Agent { .. })
        }

        pub fn is_agent_blocked(&self) -> bool {
            matches!(self, Self::Agent { is_blocked: true, .. })
        }

        pub fn is_user_in_control(&self) -> bool {
            matches!(self, Self::User { .. })
        }

        pub fn should_hide_responses(&self) -> bool {
            matches!(self, Self::Agent { should_hide_responses: true, .. })
        }

        pub fn user_take_over_reason(&self) -> Option<&UserTakeOverReason> {
            match self {
                LongRunningCommandControlState::Agent { .. } => None,
                LongRunningCommandControlState::User { reason } => Some(reason),
            }
        }
    }

    pub enum CLISubagentEvent {
        SpawnedSubagent {
            task_id: TaskId,
            block_id: BlockId,
            conversation_id: AIConversationId,
            initial_requested_command_action_id: Option<AIAgentActionId>,
        },
        FinishedSubagent {
            block_id: BlockId,
            conversation_id: Option<AIConversationId>,
            initial_requested_command_action_id: Option<AIAgentActionId>,
        },
        UpdatedControl {
            block_id: BlockId,
            requested_command_action_id: Option<AIAgentActionId>,
            agent_has_control: bool,
        },
        UpdatedLastSnapshot,
        ToggledHideResponses,
        ControlHandedBackAfterTransfer,
    }

    pub struct CLISubagentController;

    impl CLISubagentController {
        pub fn new(
            _terminal_model: Arc<FairMutex<TerminalModel>>,
            _model_event_dispatcher: &ModelHandle<ModelEventDispatcher>,
            _terminal_view_id: EntityId,
            _ctx: &mut ModelContext<Self>,
        ) -> Self {
            CLISubagentController
        }

        pub fn is_agent_in_control(&self) -> bool {
            false
        }

        pub fn last_snapshot_at(
            &self,
            _block_id: &BlockId,
        ) -> Option<instant::Instant> {
            None
        }

        pub fn switch_control_to_user(
            &self,
            _reason: UserTakeOverReason,
            _ctx: &mut ModelContext<Self>,
        ) {
        }

        pub fn handoff_active_command_control_to_agent(
            &self,
            _ctx: &mut ModelContext<Self>,
        ) {
        }

        pub fn toggle_hide_responses(&self, _ctx: &mut ModelContext<Self>) {}

        pub fn is_agent_in_control_or_tagged_in(&self) -> bool { false }
    }

    impl Entity for CLISubagentController {
        type Event = CLISubagentEvent;
    }
}

// ─── model ────────────────────────────────────────────────────────────────────

pub mod model {
    use warpui::{AppContext, ViewContext};

    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::agent::{
        AIAgentExchangeId, AIAgentInput, CancellationReason, RenderableAIError, ServerOutputId,
        Shared,
    };
    use crate::ai::agent::AIAgentOutput;
    use crate::ai::agent::PassiveSuggestionTriggerType;
    use crate::ai::llms::LLMId;

    pub type OutputStatusUpdateCallback<V> = Box<dyn FnMut(&mut V, &mut ViewContext<V>)>;

    #[derive(Debug, Clone, Copy)]
    pub enum PassiveRequestType {
        UnitTestSuggestion,
        CodeDiff,
        PassiveSuggestion(PassiveSuggestionTriggerType),
    }

    #[derive(Default, Debug, Clone, Copy)]
    pub enum AIRequestType {
        #[default]
        Active,
        Passive(PassiveRequestType),
    }

    pub enum AIBlockOutputStatus {
        Pending,
        PartiallyReceived { output: Shared<AIAgentOutput> },
        Complete { output: Shared<AIAgentOutput> },
        Cancelled {
            partial_output: Option<Shared<AIAgentOutput>>,
            reason: CancellationReason,
        },
        Failed {
            partial_output: Option<Shared<AIAgentOutput>>,
            error: RenderableAIError,
        },
    }

    impl AIBlockOutputStatus {
        pub fn is_streaming(&self) -> bool {
            matches!(
                self,
                AIBlockOutputStatus::Pending | AIBlockOutputStatus::PartiallyReceived { .. }
            )
        }

        pub fn is_cancelled(&self) -> bool {
            matches!(self, AIBlockOutputStatus::Cancelled { .. })
        }

        pub fn cancellation_reason(&self) -> Option<&CancellationReason> {
            match self {
                AIBlockOutputStatus::Cancelled { reason, .. } => Some(reason),
                _ => None,
            }
        }

        pub fn is_complete(&self) -> bool {
            matches!(self, AIBlockOutputStatus::Complete { .. })
        }

        pub fn output_to_render(&self) -> Option<Shared<AIAgentOutput>> {
            match self {
                AIBlockOutputStatus::Pending => None,
                AIBlockOutputStatus::PartiallyReceived { output } => Some(output.get_owned()),
                AIBlockOutputStatus::Complete { output } => Some(output.get_owned()),
                AIBlockOutputStatus::Cancelled { partial_output, .. } => {
                    partial_output.as_ref().map(Shared::get_owned)
                }
                AIBlockOutputStatus::Failed { partial_output, .. } => {
                    partial_output.as_ref().map(Shared::get_owned)
                }
            }
        }

        pub fn error(&self) -> Option<&RenderableAIError> {
            match self {
                AIBlockOutputStatus::Failed { error, .. } => Some(error),
                _ => None,
            }
        }
    }

    pub trait AIBlockModel {
        type View;

        fn status(&self, app: &AppContext) -> AIBlockOutputStatus;
        fn server_output_id(&self, app: &AppContext) -> Option<ServerOutputId>;
        fn model_id(&self, app: &AppContext) -> Option<LLMId>;
        fn is_restored(&self) -> bool {
            false
        }
        fn is_forked(&self) -> bool {
            false
        }
        fn was_autodetected_ai_query(&self, _app: &AppContext) -> bool {
            false
        }
        fn time_since_request_start(
            &self,
            _app: &AppContext,
        ) -> Option<chrono::TimeDelta> {
            None
        }
        fn base_model<'a>(&'a self, app: &'a AppContext) -> Option<&'a LLMId>;
        fn inputs_to_render<'a>(&'a self, app: &'a AppContext) -> &'a [AIAgentInput];
        fn conversation_id(&self, app: &AppContext) -> Option<AIConversationId>;
        fn exchange_id(&self, _app: &AppContext) -> Option<AIAgentExchangeId> {
            None
        }
        fn on_updated_output(
            &self,
            callback: OutputStatusUpdateCallback<Self::View>,
            ctx: &mut ViewContext<Self::View>,
        );
        fn request_type(&self, app: &AppContext) -> AIRequestType;
        fn is_first_action_in_output(&self, _action_id: &crate::ai::agent::AIAgentActionId, _app: &AppContext) -> bool { false }
        fn conversation<'a>(&'a self, _app: &'a AppContext) -> Option<&'a crate::ai::agent::conversation::AIConversation> { None }
    }

    impl AIRequestType {
        pub fn is_passive_code_diff(&self) -> bool { false }
    }

    pub struct AIBlockModelHelper;

    pub struct AIBlockModelImpl<V> {
        _phantom: std::marker::PhantomData<V>,
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub mod testing {
        use warpui::{AppContext, ViewContext};

        use super::{
            AIBlockModel, AIBlockOutputStatus, AIRequestType, OutputStatusUpdateCallback,
        };
        use crate::ai::agent::conversation::AIConversationId;
        use crate::ai::agent::{
            AIAgentExchangeId, AIAgentInput, AIAgentOutput, ServerOutputId, Shared,
        };
        use super::super::AIBlock;
        use crate::ai::llms::LLMId;

        pub struct FakeAIBlockModel {
            input: Vec<AIAgentInput>,
            output: Shared<AIAgentOutput>,
            model_id: LLMId,
        }

        impl FakeAIBlockModel {
            pub fn new(input: Vec<AIAgentInput>, output: AIAgentOutput) -> Self {
                Self {
                    input,
                    output: Shared::new(output),
                    model_id: "fake-llm".to_owned().into(),
                }
            }
        }

        impl AIBlockModel for FakeAIBlockModel {
            type View = AIBlock;

            fn status(&self, _app: &AppContext) -> AIBlockOutputStatus {
                AIBlockOutputStatus::Complete {
                    output: self.output.clone(),
                }
            }

            fn server_output_id(&self, _app: &AppContext) -> Option<ServerOutputId> {
                None
            }

            fn model_id(&self, _app: &AppContext) -> Option<LLMId> {
                Some(self.model_id.clone())
            }

            fn base_model<'a>(&'a self, _app: &'a AppContext) -> Option<&'a LLMId> {
                Some(&self.model_id)
            }

            fn inputs_to_render<'a>(
                &'a self,
                _app: &'a AppContext,
            ) -> &'a [AIAgentInput] {
                &self.input
            }

            fn conversation_id(&self, _app: &AppContext) -> Option<AIConversationId> {
                None
            }

            fn exchange_id(&self, _app: &AppContext) -> Option<AIAgentExchangeId> {
                None
            }

            fn on_updated_output(
                &self,
                _callback: OutputStatusUpdateCallback<Self::View>,
                _ctx: &mut ViewContext<Self::View>,
            ) {
            }

            fn request_type(&self, _app: &AppContext) -> AIRequestType {
                AIRequestType::Active
            }
        }
    }
}

// ─── AutonomySettingSpeedbump ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AutonomySettingSpeedbump {
    AutoexecuteReadonlyCommands,
    ShouldShowForAutoexecutingReadonlyCommands {
        action_id: crate::ai::agent::AIAgentActionId,
        checked: bool,
        shown: std::sync::Arc<parking_lot::Mutex<bool>>,
    },
    ShouldShowForProfileCommandAutoexecution {
        action_id: crate::ai::agent::AIAgentActionId,
        shown: std::sync::Arc<parking_lot::Mutex<bool>>,
    },
}

// ─── PendingUserQueryBlock ────────────────────────────────────────────────────

pub struct PendingUserQueryBlock;

impl Entity for PendingUserQueryBlock {
    type Event = ();
}

impl View for PendingUserQueryBlock {
    fn ui_name() -> &'static str { "PendingUserQueryBlock" }
    fn render(&self, _app: &AppContext) -> Box<dyn Element> { Empty::new().finish() }
}

impl PendingUserQueryBlock {
    pub fn clear_selection(&mut self, _app: &AppContext) {}
    pub fn selected_text(&self, _app: &AppContext) -> Option<String> { None }
}

// ─── find sub-module ──────────────────────────────────────────────────────────

pub mod find {
    #[derive(Default, Clone, Debug)]
    pub struct FindState;
}

// ─── compact_agent_input sub-module ──────────────────────────────────────────

pub mod compact_agent_input {
    use warpui::elements::Empty;
    use warpui::{AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle};

    pub enum CompactAgentInputEvent {
        Submit(String),
        Escape,
    }

    #[derive(Debug, Clone)]
    pub enum CompactAgentInputAction {}

    pub struct CompactAgentInput;

    impl CompactAgentInput {
        pub fn new(_ctx: &mut ViewContext<Self>) -> Self { CompactAgentInput }
        pub fn editor(&self) -> &Self { self }
        pub fn read<F, R>(&self, _ctx: &AppContext, f: F) -> R where F: FnOnce(&Self, &AppContext) -> R { f(self, _ctx) }
        pub fn trim(&self) -> &str { "" }
        pub fn is_empty(&self, _app: &AppContext) -> bool { true }
        pub fn buffer_text(&self, _app: &AppContext) -> String { String::new() }
        pub fn set_text(&mut self, _text: &str, _ctx: &mut ViewContext<Self>) {}
        pub fn set_placeholder_text(&mut self, _text: &str, _ctx: &mut ViewContext<Self>) {}
    }

    impl Entity for CompactAgentInput {
        type Event = CompactAgentInputEvent;
    }

    impl View for CompactAgentInput {
        fn ui_name() -> &'static str { "CompactAgentInput" }
        fn render(&self, _app: &AppContext) -> Box<dyn Element> { Empty::new().finish() }
    }

    impl TypedActionView for CompactAgentInput {
        type Action = CompactAgentInputAction;
        fn handle_action(&mut self, _: &CompactAgentInputAction, _: &mut ViewContext<Self>) {}
    }
}

// ─── number_shortcut_buttons sub-module ──────────────────────────────────────

pub mod number_shortcut_buttons {
    use warpui::elements::{Empty, MouseStateHandle};
    use warpui::{AppContext, Element, Entity, TypedActionView, View, ViewContext};
    use warpui::ui_components::button::Button;

    pub fn init(_app: &mut AppContext) {}

    pub struct NumberShortcutButtonBuilder;

    pub struct NumberShortcutButtonsConfig;

    impl NumberShortcutButtonsConfig {
        pub fn new() -> Self { NumberShortcutButtonsConfig }
        pub fn with_keyboard_navigation(self) -> Self { self }
        pub fn with_enter_to_activate(self, _activate: bool) -> Self { self }
        pub fn with_scroll_state(self, _state: impl std::any::Any) -> Self { self }
    }

    impl Default for NumberShortcutButtonsConfig {
        fn default() -> Self { Self::new() }
    }

    #[derive(Debug, Clone)]
    pub enum NumberShortcutButtonsAction {}

    pub struct NumberShortcutButtons {
        selected_button_index: Option<usize>,
    }

    impl NumberShortcutButtons {
        pub fn new_with_config(
            _buttons: Vec<NumberShortcutButtonBuilder>,
            _selected: Option<usize>,
            _config: NumberShortcutButtonsConfig,
            _ctx: &mut ViewContext<Self>,
        ) -> Self {
            NumberShortcutButtons { selected_button_index: None }
        }

        pub fn selected_button_index(&self) -> Option<usize> {
            self.selected_button_index
        }
    }

    impl Entity for NumberShortcutButtons {
        type Event = ();
    }

    impl View for NumberShortcutButtons {
        fn ui_name() -> &'static str { "NumberShortcutButtons" }
        fn render(&self, _app: &AppContext) -> Box<dyn Element> { Empty::new().finish() }
    }

    impl TypedActionView for NumberShortcutButtons {
        type Action = NumberShortcutButtonsAction;
        fn handle_action(&mut self, _: &NumberShortcutButtonsAction, _: &mut ViewContext<Self>) {}
    }

    pub fn numbered_shortcut_button<A: Clone + std::fmt::Debug + 'static>(
        _number: usize,
        _label: String,
        _is_checked: bool,
        _recommended: bool,
        _show_checkmark: bool,
        _mouse_state: MouseStateHandle,
        _action: A,
    ) -> NumberShortcutButtonBuilder {
        NumberShortcutButtonBuilder
    }

    pub fn inline_input_shortcut_button(
        _number: usize,
        _input: warpui::ViewHandle<super::compact_agent_input::CompactAgentInput>,
        _mouse_state: MouseStateHandle,
    ) -> NumberShortcutButtonBuilder {
        NumberShortcutButtonBuilder
    }
}

// ─── view_impl sub-module ─────────────────────────────────────────────────────

pub mod view_impl {
    use warpui::elements::{Empty, MouseStateHandle};
    use warpui::{AppContext, Element};

    pub use crate::terminal::view::with_content_item_spacing::{
        WithContentItemSpacing, CONTENT_HORIZONTAL_PADDING, CONTENT_ITEM_VERTICAL_MARGIN,
    };

    pub struct FindContext<'a> {
        pub model: &'a crate::terminal::find::TerminalFindModel,
        pub state: &'a crate::ai::blocklist::block::find::FindState,
    }

    pub fn render_autonomy_checkbox_setting_speedbump_footer(
        _label: &str,
        _checked: bool,
        _action: crate::ai::blocklist::block::AIBlockAction,
        _checkbox_handle: MouseStateHandle,
        _link_handle: MouseStateHandle,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        Empty::new().finish()
    }

    pub fn render_autonomy_dropdown_setting_speedbump_footer<T>(
        _label: &str,
        _dropdown: T,
        _link_handle: MouseStateHandle,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        Empty::new().finish()
    }

    pub fn render_citation(
        _citation: &crate::ai::agent::AIAgentCitation,
        _mouse_state: MouseStateHandle,
        _font_size: f32,
        _padding: f32,
        _app: &AppContext,
    ) -> Option<Box<dyn Element>> {
        None
    }

    pub fn render_citation_chips(
        _citations: &[crate::ai::agent::AIAgentCitation],
        _handles: &std::collections::HashMap<crate::ai::agent::AIAgentCitation, MouseStateHandle>,
        _font_size: f32,
        _padding: f32,
        _app: &AppContext,
    ) -> Option<Box<dyn Element>> {
        None
    }

    pub mod output {
        use warpui::elements::Empty;
        use warpui::{AppContext, Element};

        pub struct RenderContext<'a> {
            pub shell_launch_data: Option<&'a crate::terminal::ShellLaunchData>,
            pub current_working_directory: Option<&'a String>,
            pub detected_links_state: &'a crate::util::link_detection::DetectedLinksState,
            pub secret_redaction_state: &'a crate::ai::blocklist::block::secret_redaction::SecretRedactionState,
        }

        pub struct RenderReadFileArg;

        impl RenderReadFileArg {
            pub fn new<A: warpui::Action>(
                _ctx: RenderContext<'_>,
                _find_ctx: Option<super::FindContext<'_>>,
                _is_selecting: bool,
                _link_actions: crate::util::link_detection::LinkActionConstructors<A>,
            ) -> Self { RenderReadFileArg }
        }

        pub fn action_icon(
            _action_id: &crate::ai::agent::AIAgentActionId,
            _app: &AppContext,
        ) -> warpui::elements::Icon {
            warpui::elements::Icon::new("", pathfinder_color::ColorU::black())
        }

        pub fn render_read_files_text(
            _args: RenderReadFileArg,
            _file_texts: impl Iterator<Item = String>,
            _app: &AppContext,
            _appearance: &warp_core::ui::appearance::Appearance,
            _action_index: usize,
        ) -> Empty {
            Empty::new()
        }

        pub fn are_all_text_sections_empty(
            _sections: &[crate::ai::agent::AIAgentTextSection],
        ) -> bool {
            true
        }
    }
}

// ─── AIBlockResponseRating ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum AIBlockResponseRating {
    Positive,
    Negative,
}

// ─── AIBlockEvent ────────────────────────────────────────────────────────────

pub enum AIBlockEvent {
    Finished,
    UsageFooterToggled {
        conversation_id: crate::ai::agent::conversation::AIConversationId,
        is_expanded: bool,
    },
    ActionBlockedOnUserConfirmation,
    UpdateInlineActionVisibility {
        action_id: crate::ai::agent::AIAgentActionId,
        is_visible: bool,
    },
    ToggleCodeDiffVisibility,
    OpenCodeWithDiff,
    #[cfg(feature = "local_fs")]
    OpenDetectedFilePath {
        absolute_path: std::path::PathBuf,
        line_and_column_num: Option<warp_util::path::LineAndColumnArg>,
        target_override: Option<crate::util::openable_file_type::FileTarget>,
    },
    ShowLinkTooltip(crate::terminal::view::RichContentLinkTooltipInfo),
    DismissLinkTooltip,
    ShowSecretTooltip(crate::terminal::model::secrets::RichContentSecretTooltipInfo),
    DismissSecretTooltip,
    ChildViewTextSelected,
}

// ─── AIBlockAction ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AIBlockAction {
    SelectText,
    CopyAIBlockCodeSnippet(String),
    ContinueConversation,
    ResumeConversation,
    ForkConversation,
    CopyQuery,
    CopyOutput,
    Copy,
    CopyCommand,
    ToggleIsUsageFooterExpanded,
    OpenSecretTooltip {
        secret_range: warpui::elements::SecretRange,
        location: TextLocation,
    },
    ChangedHoverOnSecret {
        secret_range: warpui::elements::SecretRange,
        location: TextLocation,
        is_hovering: bool,
    },
    DismissSecretTooltip,
    ExecuteNextPendingAction,
    ToggleAutoexecuteReadonlyCommandsSpeedbumpCheckbox,
    StoreRightClickedCommand { command: String },
}

// ─── RequestedEditResolution ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RequestedEditResolution {
    Reject,
}

// ─── FinishReason ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishReason {
    Complete,
    Error,
    Cancelled,
    CancelledDuringRequestedCommandExecution,
}

// ─── ImportedComments ─────────────────────────────────────────────────────────

pub struct ImportedComments {
    pub comments: Vec<crate::code_review::comments::AttachedReviewComment>,
    pub base_branch: Option<String>,
}

// ─── AIBlock ─────────────────────────────────────────────────────────────────

pub struct AIBlock;

impl AIBlock {
    pub fn is_passive_conversation(&self, _app: &AppContext) -> bool {
        false
    }

    pub fn status(&self, _app: &AppContext) -> model::AIBlockOutputStatus {
        model::AIBlockOutputStatus::Pending
    }

    pub fn cleanup_block(&mut self, _ctx: &mut ViewContext<Self>) {}
    pub fn clear_all_selections(&mut self, _app: &AppContext) {}
    pub fn collect_imported_comments(&self) -> Option<ImportedComments> { None }
    pub fn conversation_id(&self) -> Option<crate::ai::agent::conversation::AIConversationId> { None }
    pub fn dismiss_ai_tooltips(&mut self, _ctx: &mut ViewContext<Self>) {}
    pub fn find_undismissed_code_diff(&self, _app: &AppContext) -> Option<()> { None }
    pub fn get_preceding_user_query(&self, _app: &AppContext) -> String { String::new() }
    pub fn has_any_imported_comments(&self) -> bool { false }
    pub fn hovered_rich_content_link(&self) -> Option<crate::terminal::view::RichContentLink> { None }
    pub fn is_finished(&self) -> bool { true }
    pub fn is_hidden(&self, _app: &AppContext) -> bool { false }
    pub fn is_restored(&self) -> bool { false }
    pub fn num_requested_commands(&self) -> usize { 0 }
    pub fn pending_unit_test_suggestion(&self, _app: &AppContext) -> Option<()> { None }
    pub fn requested_commands_iter<'a>(&'a self) -> impl Iterator<Item = (crate::ai::agent::AIAgentActionId, ())> + 'a { std::iter::empty() }
    pub fn revert_all_diffs(&mut self, _app: &mut AppContext) {}
    pub fn selected_text(&self, _app: &AppContext) -> Option<String> { None }
    pub fn server_output_id(&self, _app: &AppContext) -> Option<crate::ai::agent::ServerOutputId> { None }
    pub fn set_secret_redaction_state(&mut self, _location: &crate::util::text_location::TextLocation, _secret_range: &warpui::elements::SecretRange, _show_secret: bool) {}
    pub fn set_shell_launch_data(&mut self, _data: Option<crate::terminal::ShellLaunchData>, _ctx: &mut ViewContext<Self>) {}
    pub fn start_selection_at_max_point(&self, _selection_type: warpui::text::SelectionType, _x_pos: Option<f32>) {}
    pub fn start_selection_at_min_point(&self, _selection_type: warpui::text::SelectionType, _x_pos: Option<f32>) {}
}

impl Entity for AIBlock {
    type Event = AIBlockEvent;
}

impl View for AIBlock {
    fn ui_name() -> &'static str {
        "AIBlock"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl TypedActionView for AIBlock {
    type Action = AIBlockAction;

    fn handle_action(&mut self, _: &AIBlockAction, _: &mut ViewContext<Self>) {}
}

// ─── init ─────────────────────────────────────────────────────────────────────

pub fn init(_app: &mut AppContext) {}
