use std::collections::HashMap;
use std::ffi::OsString;

use warp_cli::agent::Harness;
use warpui::{EntityId, SingletonEntity, ViewContext, ViewHandle};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::llms::LLMPreferences;
use crate::pane_group::{PaneGroup, PaneId};
use crate::terminal::shared_session::IsSharedSessionCreator;
use crate::terminal::TerminalView;

pub(crate) struct HiddenChildAgentConversation {
    pub terminal_view: ViewHandle<TerminalView>,
    pub terminal_view_id: EntityId,
    pub conversation_id: AIConversationId,
}
#[derive(Clone, Debug)]
pub(crate) struct HiddenChildAgentTaskContext;

pub(crate) struct HiddenChildAgentConversationRequest {
    pub parent_pane_id: PaneId,
    pub name: String,
    pub parent_conversation_id: AIConversationId,
    pub orchestration_harness: Option<Harness>,
    pub env_vars: HashMap<OsString, OsString>,
    pub task_context: Option<HiddenChildAgentTaskContext>,
    /// When `Yes`, the child pane's terminal is asked to share its session
    /// using the embedded `SessionSourceType` once the shell bootstraps.
    /// The dispatch helpers in `terminal_pane.rs` compute this from the host
    /// terminal's own shared-session state (gated on
    /// `FeatureFlag::OrchestrationViewerPillBar`).
    pub is_shared_session_creator: IsSharedSessionCreator,
}

pub(crate) fn apply_hidden_child_agent_task_context(
    _terminal_view: &ViewHandle<TerminalView>,
    _task_context: &HiddenChildAgentTaskContext,
    _ctx: &mut ViewContext<PaneGroup>,
) {
}

fn propagate_parent_agent_settings(
    group: &PaneGroup,
    parent_pane_id: PaneId,
    child_terminal_view_id: EntityId,
    ctx: &mut ViewContext<PaneGroup>,
) {
    let Some(parent_terminal_view) = group.terminal_view_from_pane_id(parent_pane_id, ctx) else {
        log::warn!(
            "Could not find parent terminal view for pane {parent_pane_id:?}; child will use default AI profile"
        );
        return;
    };

    let parent_view_id = parent_terminal_view.id();

    let parent_base_model_id = LLMPreferences::as_ref(ctx)
        .get_active_base_model(ctx, Some(parent_view_id))
        .id
        .clone();
    LLMPreferences::handle(ctx).update(ctx, |llm_prefs, ctx| {
        llm_prefs.update_preferred_agent_mode_llm(
            &parent_base_model_id,
            child_terminal_view_id,
            ctx,
        );
    });
}

fn start_new_child_conversation(
    _terminal_view_id: EntityId,
    _name: String,
    parent_conversation_id: AIConversationId,
    _orchestration_harness: Option<Harness>,
    _ctx: &mut ViewContext<PaneGroup>,
) -> AIConversationId {
    parent_conversation_id
}

pub(crate) fn create_hidden_child_agent_conversation(
    group: &mut PaneGroup,
    request: HiddenChildAgentConversationRequest,
    ctx: &mut ViewContext<PaneGroup>,
) -> Option<HiddenChildAgentConversation> {
    let HiddenChildAgentConversationRequest {
        parent_pane_id,
        name,
        parent_conversation_id,
        orchestration_harness,
        env_vars,
        task_context,
        is_shared_session_creator,
    } = request;
    let new_pane_id = group.insert_terminal_pane_hidden_for_child_agent(
        parent_pane_id,
        env_vars,
        is_shared_session_creator,
        ctx,
    );
    let Some(new_terminal_view) = group.terminal_view_from_pane_id(new_pane_id, ctx) else {
        log::error!("Failed to get terminal view for new StartAgent pane");
        group.discard_pane(new_pane_id.into(), ctx);
        return None;
    };

    let terminal_view_id = new_terminal_view.id();
    propagate_parent_agent_settings(group, parent_pane_id, terminal_view_id, ctx);
    if let Some(task_context) = task_context.as_ref() {
        apply_hidden_child_agent_task_context(&new_terminal_view, task_context, ctx);
    }

    let conversation_id = start_new_child_conversation(
        terminal_view_id,
        name,
        parent_conversation_id,
        orchestration_harness,
        ctx,
    );

    group
        .child_agent_panes
        .insert(conversation_id, new_pane_id.into());

    Some(HiddenChildAgentConversation {
        terminal_view: new_terminal_view,
        terminal_view_id,
        conversation_id,
    })
}

