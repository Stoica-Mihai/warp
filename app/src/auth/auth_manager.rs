pub(super) mod user_persistence;

use std::sync::Arc;

use uuid::Uuid;
use warp_core::channel::ChannelState;
use warpui::{Entity, ModelContext, SingletonEntity};

use super::auth_state::{AuthState, PersistAction};
use super::auth_view_modal::AuthViewVariant;
use super::credentials::Credentials;
use super::user::User;
use super::AuthStateProvider;
use crate::server::telemetry::AnonymousUserSignupEntrypoint;
use user_persistence::PersistedUser;

#[derive(Debug)]
pub enum AuthManagerEvent {
    AttemptedLoginGatedFeature {
        auth_view_variant: AuthViewVariant,
    },
}

pub type LoginGatedFeature = &'static str;

type URLConstructorCallback = Box<dyn FnOnce(Option<&str>) -> String>;

pub struct AuthManager {
    auth_state: Arc<AuthState>,
    pending_auth_state: Option<String>,
}

impl AuthManager {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();
        Self {
            auth_state,
            pending_auth_state: None,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(ctx: &mut ModelContext<Self>) -> Self {
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();
        Self {
            auth_state,
            pending_auth_state: None,
        }
    }

    pub fn refresh_user(&self, _ctx: &mut ModelContext<Self>) {}

    pub fn authorize_device(&self, _ctx: &mut ModelContext<Self>) {}

    fn set_and_persist(
        &self,
        user: Option<User>,
        credentials: Option<Credentials>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.auth_state.set_user(user);
        self.auth_state.set_credentials(credentials);
        self.persist(ctx);
    }

    fn persist(&self, ctx: &mut ModelContext<Self>) {
        match self.auth_state.persist_action() {
            PersistAction::Persist(persisted_user) => {
                if persisted_user.auth_tokens.refresh_token.is_empty() {
                    log::warn!("Skipping user persistence due to empty refresh token");
                    return;
                }
                let _ = persisted_user.write_to_secure_storage(ctx).map_err(|err| {
                    log::warn!("Unable to persist user to secure storage: {err:?}");
                });
            }
            PersistAction::Remove => {
                let _ = PersistedUser::remove_from_secure_storage(ctx).map_err(|err| {
                    log::warn!("Unable to clear user from secure storage: {err:?}");
                });
            }
            PersistAction::DoNothing => {}
        }
    }

    pub(super) fn log_out(&mut self, ctx: &mut ModelContext<Self>) {
        self.pending_auth_state = None;
        self.set_and_persist(None, None, ctx);
    }

    pub fn attempt_login_gated_feature(
        &self,
        _feature: LoginGatedFeature,
        auth_view_variant: AuthViewVariant,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.auth_state.is_anonymous_or_logged_out() {
            ctx.emit(AuthManagerEvent::AttemptedLoginGatedFeature { auth_view_variant });
        };
    }

    pub fn anonymous_user_hit_drive_object_limit(&self, ctx: &mut ModelContext<Self>) {
        if self.auth_state.is_anonymous_or_logged_out() {
            ctx.emit(AuthManagerEvent::AttemptedLoginGatedFeature {
                auth_view_variant: AuthViewVariant::HitDriveObjectLimitCloseable,
            });
        };
    }

    pub fn initiate_anonymous_user_linking(
        &self,
        _entrypoint: AnonymousUserSignupEntrypoint,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn open_url_maybe_with_anonymous_token(
        &self,
        ctx: &mut ModelContext<Self>,
        construct_url: URLConstructorCallback,
    ) {
        ctx.open_url(&construct_url(None));
    }

    fn generate_auth_state(&mut self) -> String {
        let state = Uuid::new_v4().to_string();
        self.pending_auth_state = Some(state.clone());
        state
    }

    pub fn sign_up_url(&mut self) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/signup/remote?scheme={}&state={}&public_beta=true",
            ChannelState::server_root_url(),
            ChannelState::url_scheme(),
            state,
        )
    }

    pub fn sign_in_url(&mut self) -> String {
        let state = self.generate_auth_state();
        format!(
            "{}/login/remote?scheme={}&state={}",
            ChannelState::server_root_url(),
            ChannelState::url_scheme(),
            state,
        )
    }
}

#[derive(Clone, Debug)]
pub struct PersistedCurrentUserInformation {
    pub email: String,
}

impl Entity for AuthManager {
    type Event = AuthManagerEvent;
}

impl SingletonEntity for AuthManager {}

#[cfg(test)]
#[path = "auth_manager_tests.rs"]
mod tests;
