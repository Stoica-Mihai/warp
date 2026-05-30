use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use cynic::{MutationBuilder, QueryBuilder};
use instant::Duration;
#[cfg(test)]
use mockall::{automock, predicate::*};
use warp_graphql::client::Operation;
use warp_graphql::mutations::create_anonymous_user::{
    AnonymousUserType, CreateAnonymousUser, CreateAnonymousUserResult, CreateAnonymousUserVariables,
};
use warp_graphql::mutations::expire_api_key::{
    ExpireApiKey, ExpireApiKeyResult, ExpireApiKeyVariables,
};
use warp_graphql::mutations::generate_api_key::{
    GenerateApiKey, GenerateApiKeyInput, GenerateApiKeyResult, GenerateApiKeyVariables,
};
use warp_graphql::mutations::update_user_settings::{
    UpdateUserSettings, UpdateUserSettingsInput, UpdateUserSettingsResult,
    UpdateUserSettingsVariables,
};
use warp_graphql::queries::api_keys::{
    ApiKeyProperties, ApiKeyPropertiesResult, ApiKeys, ApiKeysVariables,
};
use warp_graphql::queries::get_conversation_usage::{
    ConversationUsage, GetConversationUsage, GetConversationUsageVariables, UserResult,
};
use warp_graphql::queries::get_user_settings::{GetUserSettings, GetUserSettingsVariables};

use super::ServerApi;
use crate::auth::credentials::{AuthToken, Credentials};
use crate::server::graphql::{
    default_request_options, get_request_context, get_user_facing_error_message,
};
use crate::server::ids::ApiKeyUid;
use crate::settings::PrivacySettingsSnapshot;

/// A named agent identity from the public API.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AgentIdentity {
    pub uid: String,
    pub name: String,
    pub available: bool,
}

/// Wrapper for the `GET /api/v1/agent/identities` response.
#[derive(serde::Deserialize)]
struct AgentIdentitiesResponse {
    agents: Vec<AgentIdentity>,
}

/// Header key for the ambient workload token attached to multi-agent requests.
pub const AMBIENT_WORKLOAD_TOKEN_HEADER: &str = "X-Warp-Ambient-Workload-Token";

/// Header key for the cloud agent task ID attached to requests from ambient agents.
pub const CLOUD_AGENT_ID_HEADER: &str = "X-Warp-Cloud-Agent-ID";

/// Duration for which the ambient workload token is valid (3 hours).
const AMBIENT_WORKLOAD_TOKEN_DURATION: Duration = Duration::from_secs(3 * 60 * 60);

/// User settings that are currently 'synced' (e.g. stored server-side) on a per-user basis.
#[derive(Copy, Clone, Debug, Default)]
pub struct SyncedUserSettings {
    pub is_cloud_conversation_storage_enabled: bool,
    pub is_crash_reporting_enabled: bool,
    pub is_telemetry_enabled: bool,
}

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AuthClient: 'static + Send + Sync {
    /// Creates an anonymous user, who is allowed to use Warp but may lack the ability
    /// to interact with particular features.
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult>;

    /// Returns the cached access token, if it is still valid. If it has expired, fetches a new
    /// access token using the user's refresh token, caches it, and the returns it.
    /// Returns an auth mode that may not require an Authorization header (e.g. session cookies or
    /// test credentials).
    async fn get_or_refresh_access_token(&self) -> Result<AuthToken>;

    /// Upon success, returns an `Option` containing the user's settings retrieved from the server,
    /// if any. The user may not have server-side settings if they onboarded prior to the launch
    /// of telemetry opt-out, have not logged in since the launch, and have never changed defaults
    /// for any of the settings in [`SyncedUserSettings`]. If the fetched settings object exists
    /// but is missing required fields, or if the request itself failed, returns an error.
    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>>;

    /// Returns conversation usage history for the current user over the past n days.
    /// If last_updated_end_timestamp is provided, only conversations with
    /// lastUpdated earlier than this timestamp are returned.
    async fn get_conversation_usage_history(
        &self,
        days: Option<i32>,
        limit: Option<i32>,
        last_updated_end_timestamp: Option<warp_graphql::scalars::Time>,
    ) -> Result<Vec<ConversationUsage>>;

    async fn set_is_telemetry_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_crash_reporting_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_cloud_conversation_storage_enabled(&self, value: bool) -> Result<()>;

    /// Sends a request to update the user's settings on the server with values contained in the
    /// given `settings_snapshot`.
    async fn update_user_settings(&self, settings_snapshot: PrivacySettingsSnapshot) -> Result<()>;

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

    /// Fetches the list of named agent identities for the user's team.
    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>>;

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
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult> {
        let variables = CreateAnonymousUserVariables {
            input: warp_graphql::mutations::create_anonymous_user::CreateAnonymousUserInput {
                anonymous_user_type,
                expiration_type: warp_graphql::mutations::create_anonymous_user::AnonymousUserExpirationType::NoExpiration,
                referral_code,
            },
            request_context: get_request_context(),
        };

        let operation = CreateAnonymousUser::build(variables);
        let response = operation
            .send_request(self.client.clone(), default_request_options())
            .await?;

        Ok(response
            .data
            .ok_or_else(|| anyhow!("missing data in response"))?
            .create_anonymous_user)
    }

    async fn get_or_refresh_access_token(&self) -> Result<AuthToken> {
        self.access_token().await
    }

    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>> {
        let variables = GetUserSettingsVariables {
            request_context: get_request_context(),
        };
        let operation = GetUserSettings::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_user_settings::UserResult::UserOutput(user_output) => {
                match user_output.user.settings {
                    Some(user_settings) => Ok(Some(SyncedUserSettings {
                        is_cloud_conversation_storage_enabled: user_settings
                            .is_cloud_conversation_storage_enabled,
                        is_crash_reporting_enabled: user_settings.is_crash_reporting_enabled,
                        is_telemetry_enabled: user_settings.is_telemetry_enabled,
                    })),
                    None => Ok(None),
                }
            }
            warp_graphql::queries::get_user_settings::UserResult::Unknown => {
                Err(anyhow!("Unable to fetch user settings"))
            }
        }
    }

    // Returns a history of the current user's conversation usage over the past n days.
    async fn get_conversation_usage_history(
        &self,
        days: Option<i32>,
        limit: Option<i32>,
        last_updated_end_timestamp: Option<warp_graphql::scalars::Time>,
    ) -> Result<Vec<ConversationUsage>> {
        let operation = GetConversationUsage::build(GetConversationUsageVariables {
            request_context: get_request_context(),
            days,
            limit,
            last_updated_end_timestamp,
        });
        let response = self.send_graphql_request(operation, None).await?;
        match response.user {
            UserResult::UserOutput(out) => Ok(out.user.conversation_usage),
            UserResult::Unknown => Err(anyhow!("Unable to fetch conversation usage")),
        }
    }

    async fn set_is_telemetry_enabled(&self, value: bool) -> Result<()> {
        let variables = UpdateUserSettingsVariables {
            input: UpdateUserSettingsInput {
                telemetry_enabled: Some(value),
                ..Default::default()
            },
            request_context: get_request_context(),
        };

        let operation = UpdateUserSettings::build(variables);
        let result = self
            .send_graphql_request(operation, None)
            .await?
            .update_user_settings;

        match result {
            UpdateUserSettingsResult::UpdateUserSettingsOutput(_) => Ok(()),
            UpdateUserSettingsResult::UserFacingError(user_facing_error) => {
                Err(anyhow!(get_user_facing_error_message(user_facing_error)))
            }
            UpdateUserSettingsResult::Unknown => Err(anyhow!("failed to set telemetry enabled")),
        }
    }

    async fn set_is_crash_reporting_enabled(&self, value: bool) -> Result<()> {
        let variables = UpdateUserSettingsVariables {
            input: UpdateUserSettingsInput {
                crash_reporting_enabled: Some(value),
                ..Default::default()
            },
            request_context: get_request_context(),
        };

        let operation = UpdateUserSettings::build(variables);
        let result = self
            .send_graphql_request(operation, None)
            .await?
            .update_user_settings;

        match result {
            UpdateUserSettingsResult::UpdateUserSettingsOutput(_) => Ok(()),
            UpdateUserSettingsResult::UserFacingError(user_facing_error) => {
                Err(anyhow!(get_user_facing_error_message(user_facing_error)))
            }
            UpdateUserSettingsResult::Unknown => {
                Err(anyhow!("failed to set crash reporting enabled"))
            }
        }
    }

    async fn set_is_cloud_conversation_storage_enabled(&self, value: bool) -> Result<()> {
        let variables = UpdateUserSettingsVariables {
            input: UpdateUserSettingsInput {
                cloud_conversation_storage_enabled: Some(value),
                ..Default::default()
            },
            request_context: get_request_context(),
        };

        let operation = UpdateUserSettings::build(variables);
        let result = self
            .send_graphql_request(operation, None)
            .await?
            .update_user_settings;

        match result {
            UpdateUserSettingsResult::UpdateUserSettingsOutput(_) => Ok(()),
            UpdateUserSettingsResult::UserFacingError(user_facing_error) => {
                Err(anyhow!(get_user_facing_error_message(user_facing_error)))
            }
            UpdateUserSettingsResult::Unknown => {
                Err(anyhow!("failed to set cloud conversation storage enabled"))
            }
        }
    }

    async fn update_user_settings(&self, settings_snapshot: PrivacySettingsSnapshot) -> Result<()> {
        let variables = UpdateUserSettingsVariables {
            input: UpdateUserSettingsInput {
                telemetry_enabled: Some(settings_snapshot.is_telemetry_enabled()),
                crash_reporting_enabled: Some(settings_snapshot.is_crash_reporting_enabled()),
                cloud_conversation_storage_enabled: settings_snapshot
                    .cloud_conversation_storage_enabled(),
            },
            request_context: get_request_context(),
        };

        let operation = UpdateUserSettings::build(variables);
        let result = self
            .send_graphql_request(operation, None)
            .await?
            .update_user_settings;

        match result {
            UpdateUserSettingsResult::UpdateUserSettingsOutput(_) => Ok(()),
            UpdateUserSettingsResult::UserFacingError(user_facing_error) => {
                Err(anyhow!(get_user_facing_error_message(user_facing_error)))
            }
            UpdateUserSettingsResult::Unknown => Err(anyhow!("failed to update user settings")),
        }
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

    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        let response: AgentIdentitiesResponse = self.get_public_api("agent/identities").await?;
        Ok(response.agents)
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
