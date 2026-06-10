//! Representation of Warp user credentials.
//!
//! The primary representation is [`Credentials`], which is the source of truth for how a user is
//! authenticated to Warp. [`AuthToken`] is the short-lived token included in server requests.
use super::user::FirebaseAuthTokens;

/// Represents the different ways a user can authenticate with Warp.
#[derive(Clone, Debug)]
pub enum Credentials {
    /// Firebase authentication with ID token and refresh token.
    Firebase(FirebaseAuthTokens),
    /// API key for direct server authentication.
    ApiKey { key: String },
    /// Request-scoped or externally managed bearer token.
    Bearer(String),
    /// Authentication derived from an ambient browser session cookie.
    SessionCookie,
    /// Test credentials used in unit tests, integration tests, and skip_login builds.
    #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
    Test,
}

impl Credentials {
    /// Returns the API key string if this is an API key credential.
    pub fn as_api_key(&self) -> Option<&str> {
        match self {
            Credentials::ApiKey { key, .. } => Some(key),
            Credentials::Firebase(_) => None,
            Credentials::Bearer(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    /// Returns the short-lived token to use in HTTP requests to the server.
    pub fn bearer_token(&self) -> AuthToken {
        match self {
            Credentials::Firebase(tokens) => AuthToken::Firebase(tokens.id_token.clone()),
            Credentials::ApiKey { key, .. } => AuthToken::ApiKey(key.clone()),
            Credentials::Bearer(token) => AuthToken::Bearer(token.clone()),
            Credentials::SessionCookie => AuthToken::NoAuth,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => AuthToken::NoAuth,
        }
    }
}

/// Represents different types of authentication tokens.
#[derive(Debug, Clone)]
pub enum AuthToken {
    /// Firebase short-lived access token.
    Firebase(String),
    /// API key for direct server authentication.
    ApiKey(String),
    /// Request-scoped or externally managed bearer token.
    Bearer(String),
    /// No authentication token available (e.g. session cookie auth or test credentials).
    #[cfg_attr(
        not(any(test, feature = "integration_tests", feature = "skip_login")),
        allow(dead_code)
    )]
    NoAuth,
}

impl AuthToken {
    /// Returns the token string to use in an Authorization header, or `None` if auth is not
    /// header-based (e.g. session cookie) or there is no auth.
    pub fn as_bearer_token(&self) -> Option<&str> {
        match self {
            AuthToken::Firebase(token) => Some(token),
            AuthToken::ApiKey(key) => Some(key),
            AuthToken::Bearer(token) => Some(token),
            AuthToken::NoAuth => None,
        }
    }

    /// Returns the bearer token as an owned string, or `None` if auth is not header-based.
    pub fn bearer_token(&self) -> Option<String> {
        match self {
            AuthToken::Firebase(token) => Some(token.clone()),
            AuthToken::ApiKey(key) => Some(key.clone()),
            AuthToken::Bearer(token) => Some(token.clone()),
            AuthToken::NoAuth => None,
        }
    }
}

