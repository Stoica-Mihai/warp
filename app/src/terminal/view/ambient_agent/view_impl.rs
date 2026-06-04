//! [`TerminalView`]-specific implementation for ambient agent functionality.

use warp_core::ui::appearance::Appearance;
use warp_terminal::model::BlockId;
use warpui::elements::Align;
use warpui::prelude::Empty;
use warpui::{AppContext, Element, ViewContext};

use super::loading_screen::{
    render_cloud_mode_cancelled_screen, render_cloud_mode_error_screen,
    render_cloud_mode_github_auth_required_screen, render_cloud_mode_loading_screen,
};
use super::AmbientAgentViewModelEvent;
use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};
use crate::terminal::view::{Event as TerminalViewEvent, TerminalView};

const CHILD_AGENT_GITHUB_AUTH_REQUIRED_BLOCKED_ACTION: &str =
    "GitHub authentication required before starting the child agent.";

impl TerminalView {
    fn active_ambient_agent_conversation_id(&self, _ctx: &AppContext) -> Option<AIConversationId> {
        None
    }

    fn active_ambient_agent_conversation_is_child(&self, ctx: &AppContext) -> bool {
        let Some(_conversation_id) = self.active_ambient_agent_conversation_id(ctx) else {
            return false;
        };

        false
    }

    fn update_active_ambient_agent_conversation_status(
        &self,
        status: ConversationStatus,
        error_message: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(_conversation_id) = self.active_ambient_agent_conversation_id(ctx) else {
            return;
        };

        let _ = (status, error_message, ctx);
    }

    /// Handles ambient agent view model events.
    pub(in crate::terminal::view) fn handle_ambient_agent_event(
        &mut self,
        event: &AmbientAgentViewModelEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(ambient_agent_view_model) = self.ambient_agent_view_model.clone() else {
            return;
        };

        // Tear down the cloud-mode queued-prompt block on terminal / transition
        // events that replace it. Legacy `Failed`, `NeedsGithubAuth`, and `Cancelled` hand off
        // to the existing error / auth / cancelled UI; `HarnessCommandStarted` hands
        // off to the live third-party harness CLI block. Idempotent and cheap when no
        // block exists.
        let should_remove_pending_user_query = match event {
            AmbientAgentViewModelEvent::Failed { .. } => true,
            AmbientAgentViewModelEvent::NeedsGithubAuth
            | AmbientAgentViewModelEvent::Cancelled
            | AmbientAgentViewModelEvent::HarnessCommandStarted { .. }
            | AmbientAgentViewModelEvent::HandoffSnapshotUploadFailed { .. } => true,
            _ => false,
        };
        if should_remove_pending_user_query {
            self.remove_pending_user_query_block(ctx);
        }

        match event {
            AmbientAgentViewModelEvent::EnteredSetupState => {
                // Re-render to show the setup view.
                self.update_pane_configuration(ctx);
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::EnteredComposingState => {
                // Update pane configuration to show cloud indicator.
                self.update_pane_configuration(ctx);
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
            }
            AmbientAgentViewModelEvent::DispatchedAgent => {
                // Pane chrome (e.g. cloud indicator, task id) must update on viewer surfaces
                // too, so this runs above the viewer short-circuit below.
                self.update_pane_configuration(ctx);
                // Only the spawner's view handles `DispatchedAgent`. Viewer surfaces (shared
                // ambient agent session or transcript viewer) have no submitted prompt to render
                // and should not insert cloud-mode rich content here.
                let is_viewer = self.is_shared_ambient_agent_session()
                    || self.model.lock().is_conversation_transcript_viewer();
                if is_viewer {
                    ctx.notify();
                    return;
                }
                // Re-render to show loading state.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::FollowupDispatched => {
                self.update_active_ambient_agent_conversation_status(
                    ConversationStatus::InProgress,
                    None,
                    ctx,
                );
                ctx.notify();
            }
            AmbientAgentViewModelEvent::SessionReady { .. }
            | AmbientAgentViewModelEvent::ExecutionSessionReady { .. } => {
                if matches!(
                    event,
                    AmbientAgentViewModelEvent::ExecutionSessionReady { .. }
                ) {
                    self.pending_cloud_followup_task_id = None;
                }
                // Re-render to hide the loading screen now that the session is ready.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::EnvironmentSelected => {}
            AmbientAgentViewModelEvent::ProgressUpdated => {
                // Update pane header to reflect any changes (e.g., task_id being set)
                self.update_pane_configuration(ctx);
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::Failed { error_message } => {
                self.pending_cloud_followup_task_id = None;
                self.update_active_ambient_agent_conversation_status(
                    ConversationStatus::Error,
                    Some(error_message.clone()),
                    ctx,
                );


                // Re-render to show the error state.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::ShowAICreditModal => {
                ctx.notify();
            }
            AmbientAgentViewModelEvent::NeedsGithubAuth => {
                self.pending_cloud_followup_task_id = None;
                if self.active_ambient_agent_conversation_is_child(ctx) {
                    self.update_active_ambient_agent_conversation_status(
                        ConversationStatus::Blocked {
                            blocked_action: CHILD_AGENT_GITHUB_AUTH_REQUIRED_BLOCKED_ACTION
                                .to_string(),
                        },
                        None,
                        ctx,
                    );
                }
                // Re-render to show the GitHub auth required state in the footer.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::Cancelled => {
                self.pending_cloud_followup_task_id = None;
                self.update_active_ambient_agent_conversation_status(
                    ConversationStatus::Cancelled,
                    None,
                    ctx,
                );
                // Re-render to show the cancelled state in the footer.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::HarnessSelected => {
                self.update_pane_configuration(ctx);
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::ViewerHarnessResolved => {
                self.update_pane_configuration(ctx);
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::HostSelected => {}
            AmbientAgentViewModelEvent::HarnessModelSelected => {}
            AmbientAgentViewModelEvent::HarnessCommandStarted { block_id } => {
                // Stop classifying the harness block as an environment setup command, mirroring
                // the Oz path in the `AppendedExchange` handler.
                let conversation_id = self.active_ambient_agent_conversation_id(ctx);
                {
                    let mut model = self.model.lock();
                    if model
                        .block_list()
                        .is_executing_oz_environment_startup_commands()
                    {
                        model
                            .block_list_mut()
                            .finish_oz_environment_startup_commands_at_block(
                                block_id,
                                conversation_id,
                            );
                    }
                }
                // Collapse the setup-commands summary, matching the oz first-exchange behavior.
                ambient_agent_view_model.update(ctx, |model, ctx| {
                    let group_id = model.setup_command_state().current_group_id();
                    model.finish_setup_command_group(group_id, ctx);
                    model.set_setup_command_visibility(false, ctx);
                });

                // Hide the command for the CLI agent block.
                // Force a fresh viewer size report to the sharer so the harness CLI (e.g.
                // the claude TUI) starts at our terminal's actual dimensions instead of
                // whatever the sandbox PTY was sized to during setup.
                ctx.emit(TerminalViewEvent::TerminalViewStateChanged);
                ctx.notify();
            }
            AmbientAgentViewModelEvent::PendingHandoffChanged => {
                ctx.notify();
            }
            AmbientAgentViewModelEvent::HandoffSnapshotUploadFailed { .. } => {
                // The toast is surfaced by `Input`'s subscription; this just
                // triggers a re-render of pane chrome.
                ctx.notify();
            }
            AmbientAgentViewModelEvent::UpdatedSetupCommandVisibility
            | AmbientAgentViewModelEvent::AuthSecretSelected
            | AmbientAgentViewModelEvent::RunLifecycleChanged => (),
        }
    }

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
            && self.ambient_agent_view_model.as_ref().is_some_and(|model| {
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

    /// Renders the ambient agent progress view based on agent progress.
    pub(in crate::terminal::view) fn render_ambient_agent_progress(
        &self,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let Some(ambient_agent_view_model) = self.ambient_agent_view_model.as_ref() else {
            return Empty::new().finish();
        };
        let ambient_agent_model = ambient_agent_view_model.as_ref(app);
        let Some(progress) = ambient_agent_model.agent_progress() else {
            return Empty::new().finish();
        };

        // Show appropriate screen based on agent status
        let ui_state = &ambient_agent_model.ui_state;
        let screen = if ambient_agent_model.is_cancelled() {
            // Show cancelled screen
            render_cloud_mode_cancelled_screen(appearance)
        } else if let Some(auth_url) = ambient_agent_model.github_auth_url() {
            // Show GitHub auth required screen
            render_cloud_mode_github_auth_required_screen(
                auth_url,
                appearance,
                &ui_state.auth_button_mouse_state,
                app,
            )
        } else if let Some(error_message) = ambient_agent_model.error_message() {
            // Show error screen
            render_cloud_mode_error_screen(
                error_message,
                appearance,
                &ui_state.error_selection_handle,
                &ui_state.error_selected_text,
                app,
            )
        } else {
            // Show loading screen - determine the message based on progress state
            let message = progress.setup_status_text();

            render_cloud_mode_loading_screen(
                message,
                appearance,
                &ui_state.loading_shimmer_handle,
                app,
            )
        };

        // Center the screen within the terminal view
        Align::new(screen).finish()
    }


}
