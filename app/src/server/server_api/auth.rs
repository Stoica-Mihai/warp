use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use cynic::{MutationBuilder, QueryBuilder};
use instant::Duration;
#[cfg(test)]
use mockall::{automock, predicate::*};
use warp_graphql::mutations::expire_api_key::{
    ExpireApiKey, ExpireApiKeyResult, ExpireApiKeyVariables,
};
use warp_graphql::mutations::generate_api_key::{
    GenerateApiKey, GenerateApiKeyInput, GenerateApiKeyResult, GenerateApiKeyVariables,
};
use warp_graphql::queries::api_keys::{
    ApiKeyProperties, ApiKeyPropertiesResult, ApiKeys, ApiKeysVariables,
};

use super::ServerApi;
use crate::auth::credentials::{AuthToken, Credentials};
use crate::server::graphql::{get_request_context, get_user_facing_error_message};
use crate::server::ids::ApiKeyUid;

/// Header key for the ambient workload token attached to multi-agent requests.
pub const AMBIENT_WORKLOAD_TOKEN_HEADER: &str = "X-Warp-Ambient-Workload-Token";

/// Header key for the cloud agent task ID attached to requests from ambient agents.
pub const CLOUD_AGENT_ID_HEADER: &str = "X-Warp-Cloud-Agent-ID";

/// Duration for which the ambient workload token is valid (3 hours).
const AMBIENT_WORKLOAD_TOKEN_DURATION: Duration = Duration::from_secs(3 * 60 * 60);

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AuthClient: 'static + Send + Sync {
    /// Returns the cached access token, if it is still valid. If it has expired, fetches a new
    /// access token using the user's refresh token, caches it, and the returns it.
    /// Returns an auth mode that may not require an Authorization header (e.g. session cookies or
    /// test credentials).
    async fn get_or_refresh_access_token(&self) -> Result<AuthToken>;

    // API Keys
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>>;

    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        agent_uid: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult>;

    async fn expire_api_key(&self, key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult>;

    /// Returns a cached ambient workload token, or issues a new one if not present or expired.
    ///
    /// Returns `Ok(None)` if not running in an isolation platform (e.g., Namespace) or on WASM.
    async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>>;
}

impl ServerApi {
    pub(super) async fn access_token(&self) -> Result<AuthToken> {
        if cfg!(feature = "skip_login") {
            bail!("skip_login enabled; failing all authenticated requests");
        }

        let Some(credentials) = self.auth_state.credentials() else {
            bail!("missing authentication credentials");
        };

        match credentials {
            Credentials::ApiKey { key, .. } => Ok(AuthToken::ApiKey(key)),
            Credentials::Bearer(token) => Ok(AuthToken::Bearer(token)),
            // Login stripped; return cached id_token without refreshing.
            Credentials::Firebase(auth_tokens) => Ok(AuthToken::Firebase(auth_tokens.id_token)),
            Credentials::SessionCookie => Ok(AuthToken::NoAuth),
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => Ok(AuthToken::NoAuth),
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AuthClient for ServerApi {
    async fn get_or_refresh_access_token(&self) -> Result<AuthToken> {
        self.access_token().await
    }

    // API Keys
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>> {
        let variables = ApiKeysVariables {
            request_context: get_request_context(),
        };
        let operation = ApiKeys::build(variables);
        let response = self.send_graphql_request(operation, None).await?;
        match response.api_keys {
            ApiKeyPropertiesResult::ApiKeyPropertiesOutput(output) => Ok(output.api_keys),
            ApiKeyPropertiesResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            ApiKeyPropertiesResult::Unknown => Err(anyhow!("failed to fetch API keys")),
        }
    }

    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        agent_uid: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult> {
        let variables = GenerateApiKeyVariables {
            input: GenerateApiKeyInput {
                name,
                team_id,
                agent_uid,
                expires_at,
            },
            request_context: get_request_context(),
        };
        let operation = GenerateApiKey::build(variables);
        let response = self.send_graphql_request(operation, None).await?;
        Ok(response.generate_api_key)
    }

    async fn expire_api_key(&self, key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult> {
        let variables = ExpireApiKeyVariables {
            key_uid: key_uid.into(),
            request_context: get_request_context(),
        };
        let op = ExpireApiKey::build(variables);
        let res = self.send_graphql_request(op, None).await?;
        Ok(res.expire_api_key)
    }

    async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>> {
        if cfg!(target_family = "wasm") {
            return Ok(None);
        }

        // Check if we have a cached token that's still valid (with 5 minute buffer).
        // Tokens without an expiration time are always considered valid.
        {
            let cached = self.ambient_workload_token.lock();
            if let Some(ref token) = *cached {
                let is_valid = token.expires_at.is_none_or(|expires_at| {
                    chrono::Utc::now() + chrono::Duration::minutes(5) < expires_at
                });
                if is_valid {
                    return Ok(Some(token.token.clone()));
                }
            }
        }

        // Issue a new token.
        let workload_token = match warp_isolation_platform::issue_workload_token(Some(
            AMBIENT_WORKLOAD_TOKEN_DURATION,
        ))
        .await
        {
            Ok(token) => token,
            Err(warp_isolation_platform::IsolationPlatformError::NoIsolationPlatformDetected) => {
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };

        let token_str = workload_token.token.clone();

        {
            let mut cached = self.ambient_workload_token.lock();
            *cached = Some(workload_token);
        }

        Ok(Some(token_str))
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
