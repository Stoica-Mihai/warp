use warpui::{App, SingletonEntity};

use super::AuthManager;
use crate::auth::credentials::Credentials;
use crate::auth::user::FirebaseAuthTokens;
use crate::auth::{AuthStateProvider};
use crate::ServerApiProvider;

fn initialize_app(app: &mut App) {
    app.add_singleton_model(|_ctx| ServerApiProvider::new_for_test());
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.add_singleton_model(AuthManager::new_for_test);
}

#[test]
fn test_log_out_clears_pending_auth_state() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        // `log_out` clears user+credentials and then calls `persist`, which
        // routes to `PersistedUser::remove_from_secure_storage`. That requires
        // a `SecureStorage` singleton, so register a no-op one for this test.
        app.update(|ctx| {
            warpui_extras::secure_storage::register_noop("warp_test", ctx);
        });

        AuthManager::handle(&app).update(&mut app, |auth_manager, ctx| {
            let _pending = auth_manager.generate_auth_state();
            assert!(
                auth_manager.pending_auth_state.is_some(),
                "precondition: generate_auth_state should populate pending_auth_state"
            );

            auth_manager.log_out(ctx);

            assert!(
                auth_manager.pending_auth_state.is_none(),
                "log_out should clear pending_auth_state"
            );
        });
    });
}

// These two tests verify that `persist` skips writing to secure storage under certain conditions.
// They rely on the fact that no secure storage singleton is registered in the test app: if
// `write_to_secure_storage` were ever called, it would panic trying to look up the unregistered
// singleton, causing the test to fail.

#[test]
fn test_persist_skips_when_refresh_token_is_empty() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        // Override default test credentials with Firebase tokens that have an empty refresh token.
        app.update(|ctx| {
            let tokens = FirebaseAuthTokens {
                id_token: String::new(),
                refresh_token: String::new(),
                expiration_time: chrono::Utc::now().fixed_offset() + chrono::Duration::days(365),
            };
            AuthStateProvider::as_ref(ctx)
                .get()
                .set_credentials(Some(Credentials::Firebase(tokens)));
        });

        AuthManager::handle(&app).update(&mut app, |auth_manager, ctx| {
            auth_manager.persist(ctx);
        });
    });
}

#[test]
fn test_persist_skips_when_api_key_authenticated() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        app.update(|ctx| {
            AuthStateProvider::as_ref(ctx)
                .get()
                .set_credentials(Some(Credentials::ApiKey {
                    key: "wk-test-key".to_owned(),
                }));
        });

        AuthManager::handle(&app).update(&mut app, |auth_manager, ctx| {
            auth_manager.persist(ctx);
        });
    });
}
