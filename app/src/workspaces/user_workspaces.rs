
use regex::Regex;
use warp_core::features::FeatureFlag;
use warp_core::settings::{ChangeEventReason, Setting};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, Tracked};

use super::workspace::{
    AdminEnablementSetting, CustomerType, EnterpriseSecretRegex, UgcCollectionEnablementSetting,
    Workspace, WorkspaceUid,
};
use crate::auth::AuthStateProvider;
use crate::channel::ChannelState;
use crate::cloud_object::model::persistence::CloudModel;
use crate::cloud_object::{Owner, Space};
use crate::settings::{
    AISettings, AISettingsChangedEvent, CodeSettings, CodeSettingsChangedEvent, PrivacySettings,
};
use crate::workspaces::workspace::{
    AiAutonomySettings, SandboxedAgentSettings, UsageBasedPricingSettings,
};


#[derive(Debug)]
pub enum UserWorkspacesEvent {
    /// Fired whenever the set of teams the user is on changes.
    TeamsChanged,
    CodebaseContextEnablementChanged,
}

/// UserWorkspaces is a singleton model that holds workspace metadata (name, members, etc).
/// It should be used for getting information about the workspaces, teams, current teams,
/// and all other things related to operating on workspace and team data.
/// TODO: move other server_api calls to update_manager to correctly update sqlite.
pub struct UserWorkspaces {
    current_workspace_uid: Tracked<Option<WorkspaceUid>>,
    workspaces: Tracked<Vec<Workspace>>,
}

impl UserWorkspaces {
    #[cfg(test)]
    pub fn mock(
        cached_workspaces: Vec<Workspace>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            current_workspace_uid: cached_workspaces.first().map(|w| w.uid).into(),
            workspaces: cached_workspaces.into(),
        }
    }

    #[cfg(test)]
    pub fn default_mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::mock(vec![], ctx)
    }

    pub fn new(
        cached_workspaces: Vec<Workspace>,
        current_workspace_uid: Option<WorkspaceUid>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&CodeSettings::handle(ctx), |_, code_settings_event, ctx| {
            match code_settings_event {
                CodeSettingsChangedEvent::CodebaseContextEnabled { .. } => {
                    ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
                }
                _ => {}
            }
        });

        ctx.subscribe_to_model(&AISettings::handle(ctx), |_, ai_settings_event, ctx| {
            if let AISettingsChangedEvent::IsAnyAIEnabled { .. } = ai_settings_event {
                ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
            }
        });

        Self {
            current_workspace_uid: current_workspace_uid.into(),
            workspaces: cached_workspaces.into(),
        }
    }


    pub fn workspace_from_uid(&self, workspace_uid: WorkspaceUid) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.uid == workspace_uid)
    }

    pub fn workspace_from_uid_mut(
        &mut self,
        workspace_uid: WorkspaceUid,
    ) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| w.uid == workspace_uid)
    }

    /// Note that the workspace is populated with dummy data until the initial fetch
    /// completes (only workspace name/ID and workspace team's name/ID are cached in
    /// sqlite locally).
    /// Consider whether you need to wait for the results of the fetch before checking the
    /// values of other fields.
    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.current_workspace_uid
            .and_then(|workspace_uid| self.workspace_from_uid(workspace_uid))
    }

    pub fn current_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.current_workspace_uid
            .and_then(|workspace_uid| self.workspace_from_uid_mut(workspace_uid))
    }

    pub fn workspaces(&self) -> &Vec<Workspace> {
        &self.workspaces
    }

    pub fn set_current_workspace_uid(
        &mut self,
        workspace_uid: WorkspaceUid,
        ctx: &mut ModelContext<Self>,
    ) {
        *self.current_workspace_uid = Some(workspace_uid);
        self.notify_and_emit_teams_changed(ctx);
    }

    /// Returns `true` if active AI is allowed for the current workspace, based on billing config.
    ///
    /// In the future, we should store active AI enablement on the policy directly. For now, we
    /// proxy whether active AI by checking whether any active AI feature is enabled.
    // Teams are not supported in Sublight; team AI policies always fall back to unrestricted.
    pub fn is_active_ai_allowed(&self) -> bool {
        true
    }

    pub fn is_prompt_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_code_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_next_command_enabled(&self) -> bool {
        true
    }

    pub fn is_git_operations_ai_enabled(&self) -> bool {
        true
    }

    /// If voice input support is not compiled into this build, always returns `false`.
    pub fn is_voice_enabled(&self) -> bool {
        false
    }

    /// Whether BYO API key is enabled for the current user, based on the active policies.
    /// Note that the value may be incorrect if called before the team's billing metadata has been fetched.
    /// Anonymous or logged-out users are not allowed to use BYO API keys.
    pub fn is_byo_api_key_enabled(&self, app: &AppContext) -> bool {
        if AuthStateProvider::as_ref(app)
            .get()
            .is_anonymous_or_logged_out()
        {
            return false;
        }
        self.current_workspace()
            .map(|workspace| workspace.is_byo_api_key_enabled())
            .unwrap_or(false)
    }
    /// Whether custom inference endpoints are enabled for the current user.
    /// Anonymous or logged-out users are not allowed to use custom inference.
    /// Enterprise workspaces require the enterprise custom inference flag, Warp Plan, or dogfood.
    pub fn is_custom_inference_enabled(&self, app: &AppContext) -> bool {
        if AuthStateProvider::as_ref(app)
            .get()
            .is_anonymous_or_logged_out()
        {
            return false;
        }

        self.current_workspace()
            .map(|workspace| {
                workspace.billing_metadata.customer_type != CustomerType::Enterprise
                    || FeatureFlag::CustomInferenceEndpointsEnterprise.is_enabled()
                    || ChannelState::channel().is_dogfood()
            })
            .unwrap_or(true)
    }

    /// Returns the AI autonomy settings that are enforced by the workspace for all its members.
    /// If a setting is `None`, the workspace doesn't enforce a particular setting.
    pub fn ai_autonomy_settings(&self) -> AiAutonomySettings {
        AiAutonomySettings::default()
    }

    /// Returns the sandboxed agent settings enforced by the workspace, if any.
    pub fn sandboxed_agent_settings(&self) -> Option<SandboxedAgentSettings> {
        None
    }

    /// Returns true iff AI autonomy features are allowed for this client.
    /// TODO: This should be deleted soon. AI autonomy settings have been moved into organization
    /// settings (see `ai_autonomy_settings` above), but there could be an interim time where we
    /// have not set up the org settings yet for an enterprise that previously had the entire
    /// feature set disabled. To capture that case, we'll see if all the settings are `None`;
    /// if so, we'll fall back to their billing metadata's value. Once we've migrated everyone
    /// into org settings, we should remove `is_enabled` from the policy and delete this function.
    pub fn is_ai_autonomy_allowed(&self) -> bool {
        true
    }

    // Returns a Vec of the user's active spaces. Includes the "Personal Space" by default.
    // Teams are not supported in Sublight, so there are no team spaces.
    pub fn all_user_spaces(&self, ctx: &AppContext) -> Vec<Space> {
        if AuthStateProvider::as_ref(ctx)
            .get()
            .is_user_web_anonymous_user()
            .unwrap_or_default()
        {
            return vec![Space::Shared];
        }

        let mut spaces = Vec::new();

        if FeatureFlag::SharedWithMe.is_enabled()
            && CloudModel::as_ref(ctx).has_directly_shared_objects(self, ctx)
        {
            spaces.push(Space::Shared);
        }
        spaces.push(Space::Personal);

        spaces
    }

    // Returns the [`Owner`] for the user's personal drive. If the user is not authenticated, this
    // returns `None`.
    pub fn personal_drive(&self, ctx: &AppContext) -> Option<Owner> {
        AuthStateProvider::as_ref(ctx)
            .get()
            .user_id()
            .map(|user_uid| Owner::User { user_uid })
    }

    // Maps a [`Space`] into an [`Owner`], based on the user's team memberships. If the space
    // does not directly identify an owner (it's the space for shared objects), returns `None`.
    pub fn space_to_owner(&self, space: Space, ctx: &AppContext) -> Option<Owner> {
        match space {
            Space::Personal => self.personal_drive(ctx),
            Space::Shared => None,
        }
    }

    // Maps an [`Owner`] into a [`Space`]. This is always possible, as unknown owners imply the
    // shared space. Teams are not supported in Sublight, so team-owned objects map to Shared.
    pub fn owner_to_space(&self, owner: Owner, ctx: &AppContext) -> Space {
        match owner {
            Owner::User { user_uid } => {
                if !FeatureFlag::SharedWithMe.is_enabled() {
                    return Space::Personal;
                }

                let current_user = AuthStateProvider::as_ref(ctx).get().user_id();
                if Some(user_uid) == current_user {
                    Space::Personal
                } else {
                    Space::Shared
                }
            }
            Owner::Team { .. } => Space::Shared,
        }
    }

    pub fn has_workspaces(&self) -> bool {
        !self.workspaces.is_empty()
    }

    pub fn update_workspaces(&mut self, workspaces: Vec<Workspace>, ctx: &mut ModelContext<Self>) {
        *self.workspaces = workspaces;
        self.notify_and_emit_teams_changed(ctx);
    }

    fn notify_and_emit_teams_changed(&self, ctx: &mut ModelContext<Self>) {
        // PrivacySettings can't observe UserWorkspaces for updates, as it's initialized too early in
        // the app initialization flow. So, we update it manually whenever teams data changes.
        PrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
            settings.set_is_telemetry_force_enabled(self.is_telemetry_force_enabled());
            settings.set_enterprise_secret_redaction_settings(
                self.is_enterprise_secret_redaction_enabled(),
                self.get_enterprise_secret_redaction_regex_list(),
                ChangeEventReason::CloudSync,
                ctx,
            );
        });

        ctx.emit(UserWorkspacesEvent::TeamsChanged);
        ctx.emit(UserWorkspacesEvent::CodebaseContextEnablementChanged);
        ctx.notify();
    }

    pub fn usage_based_pricing_settings(&self) -> UsageBasedPricingSettings {
        self.current_workspace()
            .map(|workspace| workspace.settings.usage_based_pricing_settings.clone())
            .unwrap_or_default()
    }

    pub fn is_telemetry_force_enabled(&self) -> bool {
        false
    }

    // Teams are not supported in Sublight; team org-settings always fall back to defaults.
    pub fn is_enterprise_secret_redaction_enabled(&self) -> bool {
        false
    }

    pub fn get_enterprise_secret_redaction_regex_list(&self) -> Vec<EnterpriseSecretRegex> {
        Vec::new()
    }

    pub fn get_ugc_collection_enablement_setting(&self) -> UgcCollectionEnablementSetting {
        UgcCollectionEnablementSetting::default()
    }

    pub fn get_cloud_conversation_storage_enablement_setting(&self) -> AdminEnablementSetting {
        AdminEnablementSetting::default()
    }

    pub fn is_ai_allowed_in_remote_sessions(&self) -> bool {
        true
    }

    pub fn get_remote_session_regex_list(&self) -> Vec<Regex> {
        Vec::new()
    }

    pub fn is_anyone_with_link_sharing_enabled(&self) -> bool {
        true
    }

    pub fn is_direct_link_sharing_enabled(&self) -> bool {
        true
    }

    /// Returns the codebase context settings, taking into account the organization,
    /// global AI settings, and codebase-specific settings.
    /// Prefer this function to determine whether to show indexing-related functionality.
    pub fn is_codebase_context_enabled(&self, app: &AppContext) -> bool {
        // If the organization has an explicit setting, respect it and make user toggle irrelevant.
        // - Enable: forced ON by org, regardless of user preference.
        // - Disable: forced OFF by org.
        // - RespectUserSetting: respect the user setting.
        let org_setting = self.team_allows_codebase_context();
        let ai_globally_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);

        match org_setting {
            AdminEnablementSetting::Enable => ai_globally_enabled,
            AdminEnablementSetting::Disable => false,
            AdminEnablementSetting::RespectUserSetting => {
                ai_globally_enabled && *CodeSettings::as_ref(app).codebase_context_enabled.value()
            }
        }
    }

    pub fn default_host_slug(&self) -> Option<&str> {
        None
    }

    /// Returns the team-level agent attribution setting.
    ///
    /// Use this to decide whether the user's attribution toggle should be locked
    /// (`Enable`/`Disable`) or editable (`RespectUserSetting`).
    pub fn get_agent_attribution_setting(&self) -> AdminEnablementSetting {
        AdminEnablementSetting::default()
    }

    /// Returns only the organization-specific codebase context enablement setting.
    /// Do not use this function to determine whether codebase context is generally enabled --
    /// use `is_codebase_context_enabled` instead.
    pub fn team_allows_codebase_context(&self) -> AdminEnablementSetting {
        AdminEnablementSetting::default()
    }

}

impl Entity for UserWorkspaces {
    type Event = UserWorkspacesEvent;
}

/// Mark UserWorkspaces as global application state.
impl SingletonEntity for UserWorkspaces {}

#[cfg(test)]
#[path = "user_workspaces_tests.rs"]
mod user_workspaces_tests;
