use warpui::{
    AppContext, Entity, EntityId, ModelContext, SingletonEntity, TypedActionView, ViewHandle,
    WindowId,
};

use super::{AutoCloudHandoffTrigger, Workspace, WorkspaceAction, WorkspaceRegistry};
use crate::ai::agent::conversation::AIConversationId;
use crate::settings::AISettings;
use crate::system::{SystemStats, SystemStatsEvent};
use crate::terminal::view::TerminalView;

pub(crate) struct AutoCloudHandoffRequest {
    workspace: ViewHandle<Workspace>,
    terminal_view_id: EntityId,
    conversation_id: AIConversationId,
    trigger: AutoCloudHandoffTrigger,
}

impl AutoCloudHandoffRequest {
    fn dispatch(&self, ctx: &mut AppContext) {
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.handle_action(
                &WorkspaceAction::AutoHandoffActiveAgentToCloud {
                    terminal_view_id: self.terminal_view_id,
                    conversation_id: self.conversation_id,
                    trigger: self.trigger,
                },
                ctx,
            );
        });
    }
}
pub(crate) struct AutoCloudHandoffController {}

impl AutoCloudHandoffController {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&SystemStats::handle(ctx), |controller, event, ctx| {
            controller.handle_system_stats_event(event, ctx);
        });

        Self {}
    }

    #[cfg(all(feature = "local_fs", not(target_family = "wasm")))]
    pub(crate) fn record_handoff_failed(&mut self, _conversation_id: AIConversationId) {}

    fn handle_system_stats_event(
        &mut self,
        event: &SystemStatsEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        match event {
            SystemStatsEvent::CpuWillSleep => {
                self.trigger(AutoCloudHandoffTrigger::MacOsSleep, ctx);
            }
            SystemStatsEvent::CpuWasAwakened => {}
        }
    }

    fn trigger(&mut self, trigger: AutoCloudHandoffTrigger, ctx: &mut ModelContext<Self>) {
        if let Some(request) = self.prepare_handoff_request(trigger, ctx) {
            ctx.emit(request);
        }
    }

    fn prepare_handoff_request(
        &mut self,
        trigger: AutoCloudHandoffTrigger,
        ctx: &mut ModelContext<Self>,
    ) -> Option<AutoCloudHandoffRequest> {
        if !Self::is_trigger_enabled(trigger, ctx) {
            return None;
        }

        let (terminal_view_id, _conversation_id) = Self::last_focused_local_conversation(ctx)?;

        let (_window_id, _workspace, terminal_view) =
            Self::find_workspace_and_terminal(terminal_view_id, ctx)?;

        if terminal_view.as_ref(ctx).has_active_long_running_command() {
            return None;
        }

        None
    }

    fn last_focused_local_conversation(
        _ctx: &ModelContext<Self>,
    ) -> Option<(EntityId, AIConversationId)> {
        None
    }

    fn is_trigger_enabled(trigger: AutoCloudHandoffTrigger, ctx: &ModelContext<Self>) -> bool {
        match trigger {
            AutoCloudHandoffTrigger::MacOsSleep | AutoCloudHandoffTrigger::Uri => {
                AISettings::as_ref(ctx).is_auto_handoff_on_sleep_enabled(ctx)
            }
        }
    }
    fn find_workspace_and_terminal(
        terminal_view_id: EntityId,
        ctx: &ModelContext<Self>,
    ) -> Option<(WindowId, ViewHandle<Workspace>, ViewHandle<TerminalView>)> {
        WorkspaceRegistry::as_ref(ctx)
            .all_workspaces(ctx)
            .into_iter()
            .find_map(|(window_id, workspace)| {
                let terminal_view = workspace.as_ref(ctx).terminal_view(terminal_view_id, ctx)?;
                Some((window_id, workspace, terminal_view))
            })
    }
}

impl Entity for AutoCloudHandoffController {
    type Event = AutoCloudHandoffRequest;
}

impl SingletonEntity for AutoCloudHandoffController {}

pub(crate) fn init(app: &mut AppContext) {
    let controller = app.add_singleton_model(AutoCloudHandoffController::new);
    app.subscribe_to_model(&controller, |_, request, ctx| {
        request.dispatch(ctx);
    });
}

pub(crate) fn trigger_auto_handoff_to_cloud(
    trigger: AutoCloudHandoffTrigger,
    ctx: &mut AppContext,
) {
    AutoCloudHandoffController::handle(ctx).update(ctx, |controller, ctx| {
        controller.trigger(trigger, ctx);
    });
}
