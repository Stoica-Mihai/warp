pub mod ai;
pub mod auth;
pub mod harness_support;
pub(crate) mod presigned_upload;

use std::sync::Arc;

use auth::AuthClient;
use warp_core::context_flag::ContextFlag;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::auth::auth_state::AuthState;

/// An API wrapper struct with methods to make requests to warp-server.
///
/// Prefer NOT adding new methods directly on this struct; instead, add to one of the existing
/// client trait objects, or create your own. This helps keep `ServerApi` from being overloaded
/// with disparate types of calls, and allows you to mock methods in tests.
pub struct ServerApi {
    client: Arc<http_client::Client>,
    auth_state: Arc<AuthState>,
}

impl ServerApi {
    fn new(auth_state: Arc<AuthState>) -> Self {
        let client = Arc::new(http_client::Client::new());
        Self::new_with_parts(client, auth_state)
    }

    fn new_with_parts(client: Arc<http_client::Client>, auth_state: Arc<AuthState>) -> Self {
        Self { client, auth_state }
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        let auth_state = Arc::new(AuthState::new_for_test());
        let client = Arc::new(http_client::Client::new_for_test());
        Self::new_with_parts(client, auth_state)
    }

    #[cfg(test)]
    fn new_for_test_with_bearer_token(bearer_token: Option<String>) -> Self {
        let auth_state = Arc::new(AuthState::new_logged_out_for_test());
        if let Some(bearer_token) = bearer_token {
            auth_state.set_remote_server_bearer_token(bearer_token);
        }
        Self::new_with_parts(Arc::new(http_client::Client::new_for_test()), auth_state)
    }

    /// Returns the inner `http_client::Client` used by the `ServerApi`. Callers can use this long-lived
    /// client to make requests without having to create a new client.
    pub fn http_client(&self) -> &http_client::Client {
        &self.client
    }
}

/// A singleton entity that provides access to the global [`ServerApi`] instance,
/// or any of its implemented trait objects.
pub struct ServerApiProvider {
    server_api: Arc<ServerApi>,
}

impl ServerApiProvider {
    /// Constructs a new ServerApiProvider.
    pub fn new(auth_state: Arc<AuthState>, ctx: &mut ModelContext<Self>) -> Self {
        let mut server_api = ServerApi::new(auth_state);

        if ContextFlag::NetworkLogConsole.is_enabled() {
            super::network_logging::init(
                [Arc::get_mut(&mut server_api.client)
                    .expect("guaranteed there is only one copy of client")],
                ctx,
            );
        }

        Self {
            server_api: Arc::new(server_api),
        }
    }

    /// Constructs a new SeverApiProvider for tests.
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            server_api: Arc::new(ServerApi::new_for_test()),
        }
    }

    /// Returns a handle to the underlying [`ServerApi`] object.
    /// Prefer retrieving a specific trait object related to the methods you're calling.
    pub fn get(&self) -> Arc<ServerApi> {
        self.server_api.clone()
    }

    pub fn get_auth_client(&self) -> Arc<dyn AuthClient> {
        self.server_api.clone()
    }

    /// Returns the shared HTTP client. This client is wired into network logging
    /// and includes standard Warp request headers.
    pub fn get_http_client(&self) -> Arc<http_client::Client> {
        self.server_api.client.clone()
    }
}

impl Entity for ServerApiProvider {
    type Event = ();
}

impl SingletonEntity for ServerApiProvider {}
