pub(super) mod user_persistence;

use std::sync::Arc;


use warpui::{Entity, ModelContext, SingletonEntity};

use super::auth_state::{AuthState, PersistAction};
use super::credentials::Credentials;
use super::user::User;
use super::AuthStateProvider;
use user_persistence::PersistedUser;

pub struct AuthManager {
    auth_state: Arc<AuthState>,
}

impl AuthManager {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();
        Self { auth_state }
    }

    #[cfg(test)]
    pub fn new_for_test(ctx: &mut ModelContext<Self>) -> Self {
        let auth_state = AuthStateProvider::as_ref(ctx).get().clone();
        Self { auth_state }
    }

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
        self.set_and_persist(None, None, ctx);
    }
}

#[derive(Clone, Debug)]
pub struct PersistedCurrentUserInformation {
    pub email: String,
}

impl Entity for AuthManager {
    type Event = ();
}

impl SingletonEntity for AuthManager {}

#[cfg(test)]
#[path = "auth_manager_tests.rs"]
mod tests;
