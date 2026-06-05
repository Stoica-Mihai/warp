//! [`TerminalView`]-specific implementation for ambient agent functionality.

use warp_terminal::model::BlockId;
use warpui::ViewContext;

use crate::terminal::view::TerminalView;

impl TerminalView {
    pub(in crate::terminal::view) fn maybe_insert_setup_command_blocks(
        &mut self,
        _block_id: &BlockId,
        _ctx: &mut ViewContext<Self>,
    ) {}


    /// Enter cloud agent view from this existing session. Behavior depends on the current terminal state:
    ///
    /// 1. Already in nested cloud mode with empty convo (setup/composing): ignore.
    /// 2. Already in nested cloud mode with convo started: pop to parent terminal and start a
    ///    new cloud mode session there (siblings).
    /// 3. Not in nested cloud mode: enter cloud mode from this terminal session.
    pub(in crate::terminal::view) fn enter_cloud_agent_view(
        &mut self,
        initial_prompt: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        let is_nested_cloud_mode = self.is_nested_cloud_mode(ctx);

        // (1) If we're currently in an empty cloud mode session (setup/composing; no
        // dispatched query yet), do not allow creating a new cloud mode session.
        if is_nested_cloud_mode
            && self.ambient_agent_view_model().is_some_and(|model| {
                let model = model.as_ref(ctx);
                model.is_in_setup() || model.is_configuring_ambient_agent()
            })
        {
            return;
        }

        if is_nested_cloud_mode {
            // (2) Start a sibling cloud mode session at the terminal level.
            let Some(pane_stack) = self
                .pane_stack
                .as_ref()
                .and_then(|handle| handle.upgrade(ctx))
            else {
                log::warn!(
                    "Nested cloud mode has no pane stack; cannot pop to start sibling cloud mode session"
                );
                return;
            };

            if pane_stack.as_ref(ctx).depth() <= 1 {
                log::warn!(
                    "Nested cloud mode pane stack depth <= 1; cannot pop to start sibling cloud mode session"
                );
                return;
            }

            pane_stack.update(ctx, |stack, ctx| {
                stack.pop(ctx);
            });

            let active_view = pane_stack.as_ref(ctx).active_view().clone();
            active_view.update(ctx, |view, ctx| {
                view.enter_cloud_mode_from_session(initial_prompt, ctx);
            });

            ctx.notify();
            return;
        }

        // (3) Enter cloud mode from this terminal session.
        self.enter_cloud_mode_from_session(initial_prompt, ctx);
    }

    /// Enter cloud mode from this existing session with the given initial prompt.
    ///
    /// If called from fullscreen agent view, this defers the cloud mode start until after the
    /// agent view has exited so the resulting rich content is scoped to the terminal-level.
    pub(in crate::terminal::view) fn enter_cloud_mode_from_session(
        &mut self,
        _initial_prompt: Option<String>,
        _ctx: &mut ViewContext<Self>,
    ) {
        return;
    }

}
