//! [`TerminalView`]-specific implementation for ambient agent functionality.

use std::cell::Cell;
use std::rc::Rc;

use warp_cli::agent::Harness;
use warp_core::features::FeatureFlag;
use warp_core::ui::appearance::Appearance;
use warp_terminal::model::BlockId;
use warpui::elements::Align;
use warpui::prelude::{Empty, Vector2F};
use warpui::{
    AppContext, Element, ModelHandle, SingletonEntity, ViewContext, ViewHandle,
};

use super::loading_screen::{
    render_cloud_mode_cancelled_screen, render_cloud_mode_error_screen,
    render_cloud_mode_github_auth_required_screen, render_cloud_mode_loading_screen,
};
use super::{AmbientAgentEntryBlock, AmbientAgentViewModel, AmbientAgentViewModelEvent};
use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};
use crate::ai::AIRequestUsageModel;
use crate::pane_group::TerminalViewResources;
use crate::terminal::view::rich_content::{RichContentInsertionPosition, RichContentMetadata};
use crate::terminal::view::{Event as TerminalViewEvent, TerminalView};
use crate::terminal::CLIAgent;

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

    pub(in crate::terminal::view) fn show_out_of_credits_modal(&self, ctx: &mut ViewContext<Self>) {
        AIRequestUsageModel::handle(ctx).update(ctx, |model, ctx| {
            model.refresh_request_usage_async(ctx);
        });
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


    /// Returns `true` when the block's command is the CLI for the run's configured
    /// non-oz harness (e.g. `claude …` for [`Harness::Claude`]).
    /// Used to detect the harness-start transition at `AfterBlockStarted` time. Unlike
    /// `detect_cli_agent_from_model`, this does NOT gate on `is_active_and_long_running` —
    /// we want to classify the block as the harness session as soon as it starts, before the
    /// long-running timer would otherwise elapse.
    fn block_matches_run_harness(&self, block_id: &BlockId, ctx: &AppContext) -> bool {
        let command = {
            let model = self.model.lock();
            let Some(block) = model.block_list().block_with_id(block_id) else {
                return false;
            };
            block.command_with_secrets_obfuscated(false)
        };
        let Some(cli_agent) = CLIAgent::detect(&command, None, None, ctx) else {
            return false;
        };
        let Some(ambient_agent_view_model) = self.ambient_agent_view_model.as_ref() else {
            return false;
        };
        let selected_harness = ambient_agent_view_model.as_ref(ctx).selected_harness();
        match selected_harness {
            Harness::Oz => false,
            Harness::Claude => matches!(cli_agent, CLIAgent::Claude),
            Harness::OpenCode => matches!(cli_agent, CLIAgent::OpenCode),
            Harness::Gemini => matches!(cli_agent, CLIAgent::Gemini),
            Harness::Codex => matches!(cli_agent, CLIAgent::Codex),
            Harness::Unknown => false,
        }
    }

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
        initial_prompt: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        return;
    }

    /// Start a cloud mode session nested under this one, pushing a new pane onto this view's
    /// pane_stack and returning the pushed view + model handle. The new pane enters setup mode
    /// with `initial_prompt` (if any) pre-filled in the input.
    pub(crate) fn start_cloud_mode(
        &mut self,
        initial_prompt: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) -> Option<(ViewHandle<TerminalView>, ModelHandle<AmbientAgentViewModel>)> {
        let resources = TerminalViewResources {
            tips_completed: self.tips_completed.clone(),
            server_api: self.server_api.clone(),
            model_event_sender: self.model_event_sender.clone(),
        };

        // TODO: Use self.size_info
        let (terminal_view, terminal_manager) = super::create_cloud_mode_view(
            resources,
            Vector2F::zero(),
            ctx.window_id(),
            true, // root orchestrator viewer
            ctx,
        );

        // Only insert an ambient agent entry block once the agent is actually dispatched.
        // This avoids persisting an empty "New cloud agent" entry when the user enters cloud mode
        // but exits without sending anything.
        let Some(ambient_agent_view_model) = terminal_view
            .as_ref(ctx)
            .ambient_agent_view_model()
            .cloned()
        else {
            log::warn!("Cloud mode view was created without an ambient agent view model");
            return None;
        };
        let terminal_view_weak = terminal_view.downgrade();
        let terminal_manager_weak = terminal_manager.downgrade();
        let pane_stack = self.pane_stack.clone();
        let has_inserted_entry_block = Rc::new(Cell::new(false));

        ctx.subscribe_to_model(&ambient_agent_view_model, move |me, _, event, ctx| {
            if !matches!(event, AmbientAgentViewModelEvent::DispatchedAgent) {
                return;
            }

            if has_inserted_entry_block.get() {
                return;
            }
            has_inserted_entry_block.set(true);

            let Some(pane_stack) = pane_stack.clone() else {
                log::warn!(
                    "Pane stack not available; cannot insert ambient agent entry block for cloud mode"
                );
                return;
            };

            let Some(terminal_view) = terminal_view_weak.upgrade(ctx) else {
                return;
            };
            let Some(terminal_manager) = terminal_manager_weak.upgrade(ctx) else {
                return;
            };

            let block_terminal_view = terminal_view.clone();
            let block_terminal_manager = terminal_manager.clone();
            let block_handle = ctx.add_typed_action_view(|ctx| {
                AmbientAgentEntryBlock::new(
                    block_terminal_view,
                    block_terminal_manager,
                    pane_stack.clone(),
                    ctx,
                )
            });

            me.insert_rich_content(
                None,
                block_handle.clone(),
                Some(RichContentMetadata::AmbientAgentBlock { block_handle }),
                RichContentInsertionPosition::Append {
                    insert_below_long_running_block: false,
                },
                ctx,
            );
        });

        let pane_config = self.pane_configuration.clone();
        terminal_view.update(ctx, |view, ctx| {
            view.set_pane_configuration(pane_config);
            view.enter_ambient_agent_setup(initial_prompt, ctx);
        });

        let Some(pane_stack) = self.pane_stack.clone() else {
            log::warn!("Pane stack not available, cannot enter cloud mode");
            return None;
        };
        let Some(stack) = pane_stack.upgrade(ctx) else {
            log::warn!("Pane stack deallocated, cannot enter cloud mode");
            return None;
        };
        let pushed_view = terminal_view.clone();
        stack.update(ctx, |stack, ctx| {
            stack.push(terminal_manager, pushed_view, ctx);
        });

        Some((terminal_view, ambient_agent_view_model))
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
