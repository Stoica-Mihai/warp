pub mod ai;
pub mod auth;
pub mod harness_support;
pub(crate) mod presigned_upload;

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use auth::{AuthClient, AMBIENT_WORKLOAD_TOKEN_HEADER, CLOUD_AGENT_ID_HEADER};
use chrono::{DateTime, FixedOffset};
use instant::Instant;
use parking_lot::{Mutex, RwLock};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use warp_core::context_flag::ContextFlag;
use warp_core::errors::{register_error, AnyhowErrorExt, ErrorExt};
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::auth::auth_state::AuthState;
use crate::server::server_api::presigned_upload::HttpStatusError;
use crate::ChannelState;

pub const FETCH_CHANNEL_VERSIONS_TIMEOUT: std::time::Duration = Duration::from_secs(60);

const EXPERIMENT_ID_HEADER: &str = "X-Warp-Experiment-Id";

/// We use a special error code header `X-Warp-Error-Code` to allow the server to send
/// more specific error code information, so that the client can discern between different
/// errors with the same error code.
/// See errors/http_error_codes.go on the server for possible values.
const WARP_ERROR_CODE_HEADER: &str = "X-Warp-Error-Code";

/// An error indicating the user is out of credits. The server sends 429s to communicate this
/// state, but if Cloud Run is overloaded, it can also send 429s that aren't credit-related.
/// So we use this to distinguish between the two cases.
const WARP_ERROR_CODE_OUT_OF_CREDITS: &str = "OUT_OF_CREDITS";

/// Error code indicating the user has reached their cloud agent concurrency limit.
const WARP_ERROR_CODE_AT_CAPACITY: &str = "AT_CLOUD_AGENT_CAPACITY";

/// Header used to communicate the source of an agent run (e.g. "CLI", "GITHUB_ACTION").
pub(crate) const AGENT_SOURCE_HEADER: &str = "X-Oz-Api-Source";

#[cfg(feature = "agent_mode_evals")]
pub const EVAL_USER_ID_HEADER: &str = "X-Eval-User-ID";

/// IDs in the staging database that were created specifically for evals.
/// These users have a clean state where they haven't been referred nor have referred anyone (which causes a popup in the client).
/// DO NOT REMOVE OR CHANGE THESE USERS!
///
/// Keep this list in sync with `script/populate_agent_mode_eval_user.sql`
/// in warp-server. Those rows need to exist in the DB so the authz user loader
/// can resolve these IDs during task creation; otherwise the server will 500
/// on every eval request with a nil-deref in `UserIDFromUser`.
#[cfg(feature = "agent_mode_evals")]
const EVAL_USER_IDS: [i32; 11] = [
    2162, 2164, 2165, 2166, 2167, 2168, 2169, 2172, 2173, 2174, 2175,
];

/// ResponseType received by Client
#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
#[error("{error}")]
pub struct ClientError {
    pub error: String,
    // We unconditionally check for GitHub auth errors in any public API response. It'd be much better
    // to have the server return error codes that we can parse, but this isn't yet supported.
    // See REMOTE-666
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
}

/// Error when the user is at their cloud agent concurrency limit.
#[derive(thiserror::Error, Debug, Clone, Deserialize)]
#[error("{error} (running agents: {running_agents})")]
pub struct CloudAgentCapacityError {
    pub error: String,
    pub running_agents: i32,
}

#[derive(Deserialize, Debug)]
struct TimeResponse {
    current_time: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct ServerTime {
    time_at_fetch: DateTime<FixedOffset>,
    fetched_at: Instant,
}

impl ServerTime {
    pub fn current_time(&self) -> DateTime<FixedOffset> {
        let elapsed = chrono::Duration::from_std(self.fetched_at.elapsed())
            .expect("duration should not be bigger than limit");
        self.time_at_fetch + elapsed
    }
}

/// Wrapper for deserialization errors. This covers both:
/// * Using `serde` directly
/// * Using `reqwest` decoding utilities
#[derive(thiserror::Error, Debug)]
pub enum DeserializationError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Transport(reqwest::Error),
}

#[derive(Deserialize, Debug)]
struct OutOfCreditsResponse {
    #[serde(default, rename = "userDisplayMessage")]
    user_display_message: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AIApiError {
    #[error("Request failed due to lack of AI quota.")]
    QuotaLimit {
        user_display_message: Option<String>,
    },

    #[error("Warp is currently overloaded. Please try again later.")]
    ServerOverloaded,

    #[error("Internal error occurred at transport layer.")]
    Transport(#[source] reqwest::Error),

    #[error("Failed to deserialize API response.")]
    Deserialization(#[source] DeserializationError),

    #[error("No context found on context search.")]
    NoContextFound,

    #[error("Failed with status code {0}: {1}")]
    ErrorStatus(http::StatusCode, String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("Got error when streaming {stream_type}: {source:#}")]
    Stream {
        stream_type: &'static str,
        #[source]
        source: anyhow::Error,
    },
}

impl From<http_client::ResponseError> for AIApiError {
    fn from(err: http_client::ResponseError) -> Self {
        let http_client::ResponseError {
            source,
            headers,
            body,
        } = err;
        Self::from_response_error(source, &headers, body)
    }
}

impl From<reqwest::Error> for AIApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::from_transport_error(err)
    }
}

impl From<serde_json::Error> for AIApiError {
    fn from(err: serde_json::Error) -> Self {
        AIApiError::Deserialization(err.into())
    }
}

impl AIApiError {
    /// Converts a reqwest error to an AIApiError, using response headers to distinguish
    /// between different types of 429 errors.
    fn from_response_error(
        err: reqwest::Error,
        headers: &::http::HeaderMap,
        body: Option<String>,
    ) -> Self {
        // For HTTP 429 errors, check the X-Warp-Error-Code header to distinguish
        // between out-of-credits and server-overload.
        if err.status() == Some(http::StatusCode::TOO_MANY_REQUESTS) {
            return Self::error_for_429(headers, body);
        }

        Self::from_transport_error(err)
    }

    /// Converts a transport-level reqwest error (no HTTP response) to an AIApiError.
    fn from_transport_error(err: reqwest::Error) -> Self {
        // Unfortunately, `reqwest` reports some non-decoding errors as decoding errors (e.g.
        // unexpected disconnects or timeouts while deserializing a response body). Since we
        // render deserialization and transport errors differently, we try to detect those cases
        // here.
        if err.is_timeout() {
            return AIApiError::Transport(err);
        }
        if err.is_decode() {
            #[cfg(not(target_family = "wasm"))]
            {
                use std::error::Error as _;
                let mut source = err.source();
                while let Some(underlying) = source {
                    if underlying.is::<hyper::Error>() {
                        return AIApiError::Transport(err);
                    }

                    source = underlying.source();
                }
            }

            return AIApiError::Deserialization(DeserializationError::Transport(err));
        }

        AIApiError::Transport(err)
    }

    /// Returns the appropriate error for a 429 response by checking the X-Warp-Error-Code header.
    fn error_for_429(headers: &::http::HeaderMap, body: Option<String>) -> Self {
        if headers
            .get(WARP_ERROR_CODE_HEADER)
            .and_then(|v| v.to_str().ok())
            == Some(WARP_ERROR_CODE_OUT_OF_CREDITS)
        {
            let user_display_message = body
                .and_then(|body| serde_json::from_str::<OutOfCreditsResponse>(&body).ok())
                .and_then(|r| r.user_display_message);
            AIApiError::QuotaLimit {
                user_display_message,
            }
        } else {
            AIApiError::ServerOverloaded
        }
    }

    /// Format a stream error into a human-readable error message. This will read the response
    /// body if there is one.
    async fn from_stream_error(stream_type: &'static str, err: reqwest_eventsource::Error) -> Self {
        match err {
            reqwest_eventsource::Error::InvalidStatusCode(
                http::StatusCode::TOO_MANY_REQUESTS,
                res,
            ) => {
                let headers = res.headers().clone();
                let body = res.text().await.ok();
                Self::error_for_429(&headers, body)
            }
            reqwest_eventsource::Error::InvalidStatusCode(status, res) => Self::ErrorStatus(
                status,
                res.text()
                    .await
                    .unwrap_or_else(|e| format!("(no response body: {e:#})")),
            ),
            reqwest_eventsource::Error::Transport(err) => Self::from_transport_error(err),
            err => AIApiError::Stream {
                stream_type,
                // On WASM, `reqwest_eventsource::Error` doesn't implement `Into<anyhow::Error>` or
                // `Send` because it may contain a `wasm_bindgen` JS value.
                #[cfg(target_family = "wasm")]
                source: anyhow!("{err:#?}"),
                #[cfg(not(target_family = "wasm"))]
                source: anyhow!(err),
            },
        }
    }

    /// Returns whether or not the error can be retried.
    pub fn is_retryable(&self) -> bool {
        // Don't retry client errors, except for timeouts and quota limits.
        fn is_retryable_status(status: http::StatusCode) -> bool {
            !status.is_client_error()
                || status == http::StatusCode::REQUEST_TIMEOUT
                || status == http::StatusCode::TOO_MANY_REQUESTS
        }

        match self {
            AIApiError::ErrorStatus(status, _) => is_retryable_status(*status),
            AIApiError::Transport(e) => {
                if let Some(status) = e.status() {
                    return is_retryable_status(status);
                }
                true
            }
            // By default, retry on error.
            _ => true,
        }
    }
}

impl ErrorExt for AIApiError {
    fn is_actionable(&self) -> bool {
        match self {
            AIApiError::Deserialization(_) => true,
            AIApiError::Transport(error) => error.is_actionable(),
            AIApiError::Other(error) => error.is_actionable(),
            AIApiError::Stream { source, .. } => source.is_actionable(),
            AIApiError::ErrorStatus(_, _) => self.is_retryable(),
            AIApiError::QuotaLimit { .. }
            | AIApiError::ServerOverloaded
            | AIApiError::NoContextFound => false,
        }
    }
}
register_error!(AIApiError);

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        // The WASM version of this type has no bound on `Send`, which is not implemented on
        // `wasm_bindgen::JsValue`, which is ultimately used in reqwest_eventsource::Error. Furthermore,
        // `Send` is an unnecessary bound when targeting wasm because the browser is single-threaded (and
        // we don't leverage WebWorkers for async execution in WoW).
        pub type AIOutputStream<T> = futures::stream::LocalBoxStream<'static, Result<T, Arc<AIApiError>>>;
    } else {
        pub type AIOutputStream<T> = futures::stream::BoxStream<'static, Result<T, Arc<AIApiError>>>;
    }
}

/// An event related to the server API itself (and not a particular API call).
/// Most errors should be handled in callbacks to individual APIs, rather than sent over the
/// server API channel.
#[derive(Clone)]
pub enum ServerApiEvent {
    /// We made a staging API call that was blocked, which may indicate a firewall misconfiguration.
    StagingAccessBlocked,
    /// The user's account has been disabled.
    UserAccountDisabled,
}

impl fmt::Debug for ServerApiEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StagingAccessBlocked => f.write_str("StagingAccessBlocked"),
            Self::UserAccountDisabled => f.write_str("UserAccountDisabled"),
        }
    }
}

/// An API wrapper struct with methods to requests to warp-server.
///
/// Prefer NOT adding new methods directly on this struct; instead, add to one of the existing
/// client trait objects, or create your own. This helps keep `ServerApi` from being overloaded
/// with disparate types of calls, and allows you to mock methods in tests.
pub struct ServerApi {
    client: Arc<http_client::Client>,
    auth_state: Arc<AuthState>,
    event_sender: async_channel::Sender<ServerApiEvent>,
    last_server_time: Arc<Mutex<Option<ServerTime>>>,
    /// Cached ambient workload token for requests from ambient agents.
    ambient_workload_token: Arc<Mutex<Option<warp_isolation_platform::WorkloadToken>>>,
    /// The ambient agent task ID for requests from cloud agents.
    ambient_agent_task_id: Arc<RwLock<Option<AmbientAgentTaskId>>>,
    /// The source of agent runs (e.g. CLI, GitHub Action). Set once at startup and immutable.
    agent_source: Option<ai::AgentSource>,

    #[cfg(feature = "agent_mode_evals")]
    eval_user_id: Option<i32>,
}

impl ServerApi {
    fn new(
        auth_state: Arc<AuthState>,
        event_sender: async_channel::Sender<ServerApiEvent>,
        agent_source: Option<ai::AgentSource>,
    ) -> Self {
        let client = Arc::new(http_client::Client::new());
        Self::new_with_parts(client, auth_state, event_sender, agent_source)
    }

    fn new_with_parts(
        client: Arc<http_client::Client>,
        auth_state: Arc<AuthState>,
        event_sender: async_channel::Sender<ServerApiEvent>,
        agent_source: Option<ai::AgentSource>,
    ) -> Self {
        // We generate a random user ID for evals so we can run evals in parallel.
        #[cfg(feature = "agent_mode_evals")]
        let eval_user_id = {
            use rand::Rng;
            Some(EVAL_USER_IDS[rand::thread_rng().gen_range(0..EVAL_USER_IDS.len())])
        };

        Self {
            client,
            auth_state,
            event_sender,
            last_server_time: Arc::new(Mutex::new(None)),
            ambient_workload_token: Arc::new(Mutex::new(None)),
            ambient_agent_task_id: Arc::new(RwLock::new(None)),
            agent_source,
            #[cfg(feature = "agent_mode_evals")]
            eval_user_id,
        }
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        let (tx, _) = async_channel::unbounded();
        let auth_state = Arc::new(AuthState::new_for_test());
        let client = Arc::new(http_client::Client::new_for_test());

        Self::new_with_parts(client, auth_state, tx, None)
    }

    #[cfg(test)]
    fn new_for_test_with_bearer_token(
        bearer_token: Option<String>,
        event_sender: async_channel::Sender<ServerApiEvent>,
    ) -> Self {
        let auth_state = Arc::new(AuthState::new_logged_out_for_test());
        if let Some(bearer_token) = bearer_token {
            auth_state.set_remote_server_bearer_token(bearer_token);
        }
        Self::new_with_parts(
            Arc::new(http_client::Client::new_for_test()),
            auth_state,
            event_sender,
            None,
        )
    }

    pub fn allowed_to_refresh_token(&self) -> bool {
        self.auth_state
            .credentials()
            .is_none_or(|credentials| !credentials.is_externally_managed())
    }

    fn access_token_ignoring_validity(&self) -> Option<String> {
        self.auth_state.get_access_token_ignoring_validity()
    }

    pub(super) fn anonymous_id(&self) -> String {
        self.auth_state.anonymous_id()
    }

    /// Sets the ambient agent task ID to be sent with all subsequent requests.
    pub fn set_ambient_agent_task_id(&self, task_id: Option<AmbientAgentTaskId>) {
        *self.ambient_agent_task_id.write() = task_id;
    }

    /// Returns ambient agent headers to attach to requests.
    async fn ambient_agent_headers(&self) -> Result<Vec<(&'static str, String)>> {
        let workload_token = self
            .get_or_create_ambient_workload_token()
            .await
            .context("Failed to get ambient workload token")?;

        let task_id = self
            .ambient_agent_task_id
            .read()
            .as_ref()
            .map(|id| id.to_string());

        let agent_source = self.agent_source.as_ref().map(|s| s.as_str().to_string());

        Ok(workload_token
            .map(|token| (AMBIENT_WORKLOAD_TOKEN_HEADER, token))
            .into_iter()
            .chain(task_id.map(|id| (CLOUD_AGENT_ID_HEADER, id)))
            .chain(agent_source.map(|s| (AGENT_SOURCE_HEADER, s)))
            .collect())
    }

    async fn ambient_agent_headers_for_task(
        &self,
        task_id: &AmbientAgentTaskId,
    ) -> Result<Vec<(&'static str, String)>> {
        let mut headers = self.ambient_agent_headers().await?;
        headers.retain(|(name, _)| *name != CLOUD_AGENT_ID_HEADER);
        headers.push((CLOUD_AGENT_ID_HEADER, task_id.to_string()));
        Ok(headers)
    }


    /// Sends a GET request to a public API endpoint.
    ///
    /// # Arguments
    /// * `path` - Endpoint path relative to `/api/v1` (e.g., "agent/tasks/{task_id}")
    async fn get_public_api<R>(&self, path: &str) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let response = self.get_public_api_response(path).await?;
        let url = response.url().clone();
        response
            .json::<R>()
            .await
            .with_context(|| format!("Failed to deserialize response from {url}"))
    }

    /// Sends a GET request to a public API endpoint and returns the raw response on success.
    ///
    /// Unlike [`get_public_api`], this does not attempt JSON deserialization on the
    /// response body, allowing the caller to decode it however they need.
    async fn get_public_api_response(&self, path: &str) -> Result<http_client::Response> {
        let auth_token = self
            .get_or_refresh_access_token()
            .await
            .context("Failed to get access token for API request")?;

        let url = format!("{}/api/v1/{}", ChannelState::server_root_url(), path);

        let mut request = self.client.get(&url);
        if let Some(token) = auth_token.as_bearer_token() {
            request = request.bearer_auth(token);
        }

        for (name, value) in self.ambient_agent_headers().await? {
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("Failed to send API request to {url}"))?;

        if response.status().is_success() {
            Ok(response)
        } else {
            // Put `HttpStatusError` in the error chain so shared retry classifiers
            // (`is_transient_http_error`) can distinguish transient 5xx / 408 / 429
            // from permanent 4xx without string-matching the Display output.
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let status_err = HttpStatusError {
                status: status.as_u16(),
                body: body.clone(),
            };
            match serde_json::from_str::<ClientError>(&body) {
                Ok(error_response) => {
                    Err(anyhow::Error::new(status_err).context(error_response.error))
                }
                Err(_) => Err(anyhow::Error::new(status_err)
                    .context(format!("API request failed with status {status}"))),
            }
        }
    }


    /// Sends a POST request to a public API endpoint and returns the raw response on success.
    async fn post_public_api_response<B>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<http_client::Response>
    where
        B: Serialize,
    {
        let auth_token = self
            .get_or_refresh_access_token()
            .await
            .context("Failed to get access token for API request")?;

        let url = format!("{}/api/v1/{}", ChannelState::server_root_url(), path);

        let mut request = self.client.post(&url).json(body);
        if let Some(token) = auth_token.as_bearer_token() {
            request = request.bearer_auth(token);
        }

        for (name, value) in self.ambient_agent_headers().await? {
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("Failed to send API request to {url}"))?;

        if response.status().is_success() {
            Ok(response)
        } else {
            Err(Self::error_from_response(response).await)
        }
    }

    /// Converts a non-success public API response into the most specific client error available.
    async fn error_from_response(response: http_client::Response) -> anyhow::Error {
        let status = response.status();
        let is_at_capacity = response
            .headers()
            .get(WARP_ERROR_CODE_HEADER)
            .and_then(|v| v.to_str().ok())
            == Some(WARP_ERROR_CODE_AT_CAPACITY);
        let is_out_of_credits = response
            .headers()
            .get(WARP_ERROR_CODE_HEADER)
            .and_then(|v| v.to_str().ok())
            == Some(WARP_ERROR_CODE_OUT_OF_CREDITS);

        // Get the response text first since we may need to try multiple deserializations.
        let response_text = response.text().await.unwrap_or_default();

        // Check for AT_CAPACITY error code header.
        if is_at_capacity {
            if let Ok(capacity_error) =
                serde_json::from_str::<CloudAgentCapacityError>(&response_text)
            {
                return capacity_error.into();
            }
        }
        if status == StatusCode::TOO_MANY_REQUESTS && is_out_of_credits {
            let user_display_message = serde_json::from_str::<OutOfCreditsResponse>(&response_text)
                .ok()
                .and_then(|r| r.user_display_message);
            return AIApiError::QuotaLimit {
                user_display_message,
            }
            .into();
        }

        // Try to deserialize error response as { "error": "message" }
        match serde_json::from_str::<ClientError>(&response_text) {
            Ok(error_response) => error_response.into(),
            Err(_) => anyhow!("API request failed with status {status}"),
        }
    }

    /// Sends a POST request to a public API endpoint.
    ///
    /// # Arguments
    /// * `path` - Endpoint path relative to `/api/v1` (e.g., "agent/run")
    /// * `body` - Request body to serialize as JSON
    async fn post_public_api<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: Serialize,
        R: serde::de::DeserializeOwned,
    {
        let response = self.post_public_api_response(path, body).await?;
        let url = response.url().clone();
        response
            .json::<R>()
            .await
            .with_context(|| format!("Failed to deserialize response from {url}"))
    }




    fn set_server_time(&self, server_time: ServerTime) {
        let mut last_server_time = self.last_server_time.lock();
        *last_server_time = Some(server_time);
    }

    fn cached_server_time(&self) -> Option<ServerTime> {
        let last_server_time = self.last_server_time.lock();
        last_server_time.as_ref().cloned()
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
    pub fn new(
        auth_state: Arc<AuthState>,
        agent_source: Option<ai::AgentSource>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let (event_sender, event_receiver) = async_channel::bounded(10);
        let mut server_api = ServerApi::new(auth_state.clone(), event_sender, agent_source);

        if ContextFlag::NetworkLogConsole.is_enabled() {
            super::network_logging::init(
                [Arc::get_mut(&mut server_api.client)
                    .expect("guaranteed there is only one copy of client")],
                ctx,
            );
        }

        ctx.spawn_stream_local(
            event_receiver,
            move |_, event, ctx| {
                match event {
                    ServerApiEvent::UserAccountDisabled => {
                        // We dispatch a global action here because the log out code requires
                        // `server_api`, causing a circular model reference panic when it calls
                        // `ServerApiProvider` to get access.
                        // TODO: We should remove this pattern where `ServerApiProvider` responds
                        // to events; it's prone to these sorts of circular reference issues.
                        ctx.dispatch_global_action("app:log_out", ());
                    }
                    // Re-emit the event for subscribers.
                    // TODO: we probably want a different type for the event emitted to subscribers
                    // from the one that's used for the async channel.
                    _ => ctx.emit(event),
                }
            },
            |_, _| {},
        );
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
    type Event = ServerApiEvent;
}

impl SingletonEntity for ServerApiProvider {}
