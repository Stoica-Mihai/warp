use warpui::{ViewContext, ViewHandle};

use crate::pane_group::PaneGroup;
use crate::terminal::TerminalView;

#[derive(Clone, Debug)]
pub(crate) struct HiddenChildAgentTaskContext;

pub(crate) fn apply_hidden_child_agent_task_context(
    _terminal_view: &ViewHandle<TerminalView>,
    _task_context: &HiddenChildAgentTaskContext,
    _ctx: &mut ViewContext<PaneGroup>,
) {
}

