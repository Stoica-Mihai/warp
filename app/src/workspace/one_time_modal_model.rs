use settings::Setting as _;
use warpui::{Entity, ModelContext, SingletonEntity, WindowId};

use crate::terminal::general_settings::GeneralSettings;

/// A generic model for managing one-time modals that should be shown to users only once.
///
/// Initially implemented for the ADE launch modal, but designed to be extensible to support
/// other types of one-time modals in the future. The model holds the canonical state of whether
/// a modal is currently being shown and automatically triggers the modal when appropriate
/// conditions are met (e.g., user becomes onboarded).
pub struct OneTimeModalModel {
    is_build_plan_migration_modal_open: bool,
    target_window_id: Option<WindowId>,
}

impl OneTimeModalModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        // Subscribe to UserWorkspaces to detect when sunsetted_to_build_ts changes
        ctx.subscribe_to_model(
            &crate::workspaces::user_workspaces::UserWorkspaces::handle(ctx),
            |me, event, ctx| {
                use crate::workspaces::user_workspaces::UserWorkspacesEvent;
                if let UserWorkspacesEvent::SunsettedToBuildDataUpdated = event {
                    // When sunsetted_to_build_ts is updated, check if we should show the modal
                    me.check_and_trigger_build_plan_migration_modal(ctx);
                }
            },
        );


        Self {
            is_build_plan_migration_modal_open: false,
            target_window_id: None,
        }
    }

    /// Returns the window ID where the currently open one-time modal should be displayed.
    pub fn target_window_id(&self) -> Option<WindowId> {
        self.target_window_id
    }

    /// Returns true if any one-time modal is currently open.
    pub fn is_any_modal_open(&self) -> bool {
        self.is_build_plan_migration_modal_open && self.target_window_id.is_some()
    }

    pub fn update_target_window_id(&mut self, window_id: WindowId, ctx: &mut ModelContext<Self>) {
        let was_any_modal_visible = self.is_any_modal_open();
        self.target_window_id = Some(window_id);
        if was_any_modal_visible != self.is_any_modal_open() {
            ctx.emit(OneTimeModalEvent::VisibilityChanged {
                is_open: self.is_any_modal_open(),
            });
        }
    }

    pub fn is_build_plan_migration_modal_open(&self) -> bool {
        self.is_build_plan_migration_modal_open && self.target_window_id.is_some()
    }

    pub fn mark_build_plan_migration_modal_dismissed(&mut self, ctx: &mut ModelContext<Self>) {
        self.set_build_plan_migration_modal_open(false, ctx);
    }

    #[cfg(debug_assertions)]
    pub fn force_open_build_plan_migration_modal(&mut self, ctx: &mut ModelContext<Self>) {
        self.set_build_plan_migration_modal_open(true, ctx);
    }

    fn set_build_plan_migration_modal_open(
        &mut self,
        is_open: bool,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        if self.is_build_plan_migration_modal_open != is_open {
            self.is_build_plan_migration_modal_open = is_open;
            ctx.emit(OneTimeModalEvent::VisibilityChanged { is_open });
            return true;
        }
        false
    }

    fn check_and_trigger_build_plan_migration_modal(
        &mut self,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        use crate::workspaces::user_workspaces::UserWorkspaces;

        // Check if already dismissed
        let general_settings = GeneralSettings::as_ref(ctx);
        if *general_settings
            .build_plan_migration_modal_dismissed
            .value()
        {
            return false;
        }

        // Check if user is authenticated
        let auth_state = crate::auth::AuthStateProvider::as_ref(ctx).get();

        if auth_state.is_anonymous_or_logged_out() {
            return false;
        }

        // Check if current workspace has sunsetted_to_build_ts set
        let user_workspaces = UserWorkspaces::as_ref(ctx);
        let Some(current_team) = user_workspaces.current_team() else {
            return false;
        };

        // Check if user is admin of the team
        let Some(user_email) = auth_state.user_email() else {
            return false;
        };

        if !current_team.has_admin_permissions(&user_email) {
            return false;
        }

        // Check if service agreement has sunsetted_to_build_ts set
        let has_sunsetted_to_build = current_team
            .billing_metadata
            .service_agreements
            .first()
            .is_some_and(|sa| sa.sunsetted_to_build_ts.is_some());

        if !has_sunsetted_to_build {
            return false;
        }

        // All conditions met, show the modal
        self.set_build_plan_migration_modal_open(true, ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneTimeModalEvent {
    VisibilityChanged { is_open: bool },
}

impl Entity for OneTimeModalModel {
    type Event = OneTimeModalEvent;
}

impl SingletonEntity for OneTimeModalModel {}
