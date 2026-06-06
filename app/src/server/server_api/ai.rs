use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use cynic::{MutationBuilder, QueryBuilder};
#[cfg(test)]
use mockall::automock;
use warp_core::channel::ChannelState;
use warp_graphql::mutations::generate_metadata_for_command::{
    GenerateMetadataForCommand, GenerateMetadataForCommandInput, GenerateMetadataForCommandResult,
    GenerateMetadataForCommandStatus, GenerateMetadataForCommandVariables,
};
#[cfg(not(feature = "agent_mode_evals"))]
use warp_graphql::queries::get_request_limit_info::{
    GetRequestLimitInfo, GetRequestLimitInfoVariables,
};
use super::auth::AuthClient;
use super::ServerApi;
use crate::ai::ambient_agents::AmbientAgentTaskId;
// Re-export ambient agent types for backwards compatibility
pub use crate::ai::ambient_agents::{AgentConfigSnapshot, AgentSource};
use crate::ai::generate_code_review_content::api::{
    GenerateCodeReviewContentRequest, GenerateCodeReviewContentResponse,
};
#[cfg(feature = "agent_mode_evals")]
use crate::ai::request_usage_model::RequestLimitInfo;
#[cfg(not(feature = "agent_mode_evals"))]
use crate::ai::BonusGrant;
use crate::ai::RequestUsageInfo;
use crate::drive::workflows::ai_assist::{GeneratedCommandMetadata, GeneratedCommandMetadataError};
use crate::server::graphql::{get_request_context, get_user_facing_error_message};
#[cfg(not(feature = "agent_mode_evals"))]
use crate::{
    ai::request_usage_model::BonusGrantScope,
    server::ids::ServerId,
    workspaces::{gql_convert::PLACEHOLDER_WORKSPACE_UID, workspace::WorkspaceUid},
};

const AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS: u64 = 30;

/// Server-minted token returned by `POST /agent/handoff/upload-snapshot` that scopes a batch
/// of presigned upload URLs to `handoff/{token}/`. The client passes it
/// back via `SpawnAgentRequest.initial_snapshot_token`; the server stores it on the new run's
/// queued execution input so rehydration discovery can read the same prefix.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InitialSnapshotToken(String);

impl InitialSnapshotToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}



#[derive(Clone, serde::Deserialize, Debug, PartialEq, Eq)]
pub struct ConnectedSelfHostedWorker {
    pub worker_host: String,
    pub connection_count: u32,
    pub connected_at: String,
    pub last_seen_at: String,
}

#[derive(Clone, serde::Deserialize, Debug, PartialEq, Eq)]
pub struct ListConnectedSelfHostedWorkersResponse {
    pub workers: Vec<ConnectedSelfHostedWorker>,
}

pub(crate) const CONNECTED_SELF_HOSTED_WORKERS_PATH: &str = "agent/connected-self-hosted-workers";

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AIClient: 'static + Send + Sync {
    async fn generate_metadata_for_command(
        &self,
        command: String,
    ) -> Result<GeneratedCommandMetadata, GeneratedCommandMetadataError>;

    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error>;


    async fn list_connected_self_hosted_workers(
        &self,
    ) -> Result<ListConnectedSelfHostedWorkersResponse, anyhow::Error>;

    async fn cancel_ambient_agent_task(
        &self,
        task_id: &AmbientAgentTaskId,
    ) -> anyhow::Result<(), anyhow::Error>;

    /// Generates AI copy for code-review flows: commit messages at dialog-open
    /// time and PR titles / bodies at confirm time. `output_type` in the
    /// request picks which of the three the server returns.
    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AIClient for ServerApi {
    async fn generate_metadata_for_command(
        &self,
        command: String,
    ) -> Result<GeneratedCommandMetadata, GeneratedCommandMetadataError> {
        let default_err = GeneratedCommandMetadataError::Other;
        let variables = GenerateMetadataForCommandVariables {
            input: GenerateMetadataForCommandInput { command },
            request_context: get_request_context(),
        };

        let operation = GenerateMetadataForCommand::build(variables);
        let response = self
            .send_graphql_request(
                operation,
                Some(Duration::from_secs(AI_ASSISTANT_REQUEST_TIMEOUT_SECONDS)),
            )
            .await
            .map_err(|_| default_err)?;

        match response.generate_metadata_for_command {
            GenerateMetadataForCommandResult::GenerateMetadataForCommandOutput(output) => {
                match output.status {
                    GenerateMetadataForCommandStatus::GenerateMetadataForCommandSuccess(
                        success,
                    ) => Ok(success.into()),
                    GenerateMetadataForCommandStatus::GenerateMetadataForCommandFailure(
                        failure,
                    ) => Err(failure.type_.into()),
                    GenerateMetadataForCommandStatus::Unknown => {
                        Err(GeneratedCommandMetadataError::Other)
                    }
                }
            }
            _ => Err(GeneratedCommandMetadataError::Other),
        }
    }

    #[cfg(feature = "agent_mode_evals")]
    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error> {
        Ok(RequestUsageInfo {
            request_limit_info: RequestLimitInfo::new_for_evals(),
            bonus_grants: vec![],
        })
    }

    #[cfg(not(feature = "agent_mode_evals"))]
    async fn get_request_limit_info(&self) -> Result<RequestUsageInfo, anyhow::Error> {
        let variables = GetRequestLimitInfoVariables {
            request_context: get_request_context(),
        };
        let operation = GetRequestLimitInfo::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_request_limit_info::UserResult::UserOutput(user_output) => {
                let request_limit_info = user_output.user.request_limit_info.into();

                let workspace_bonus_grants = user_output
                    .user
                    .workspaces
                    .into_iter()
                    .filter(|workspace| workspace.uid != PLACEHOLDER_WORKSPACE_UID.into())
                    .flat_map(|workspace| {
                        let workspace_uid =
                            WorkspaceUid::from(ServerId::from_string_lossy(workspace.uid.inner()));
                        workspace
                            .bonus_grants_info
                            .grants
                            .into_iter()
                            .map(move |grant| {
                                BonusGrant::from_gql_bonus_grant(
                                    grant,
                                    BonusGrantScope::Workspace(workspace_uid),
                                )
                            })
                    });

                let bonus_grants: Vec<BonusGrant> = user_output
                    .user
                    .bonus_grants
                    .into_iter()
                    .map(|grant| BonusGrant::from_gql_bonus_grant(grant, BonusGrantScope::User))
                    .chain(workspace_bonus_grants)
                    .collect();

                Ok(RequestUsageInfo {
                    request_limit_info,
                    bonus_grants,
                })
            }
            warp_graphql::queries::get_request_limit_info::UserResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            warp_graphql::queries::get_request_limit_info::UserResult::Unknown => {
                Err(anyhow!("failed to get request limit info"))
            }
        }
    }

    async fn list_connected_self_hosted_workers(
        &self,
    ) -> anyhow::Result<ListConnectedSelfHostedWorkersResponse, anyhow::Error> {
        self.get_public_api(CONNECTED_SELF_HOSTED_WORKERS_PATH)
            .await
    }

    async fn cancel_ambient_agent_task(
        &self,
        task_id: &AmbientAgentTaskId,
    ) -> anyhow::Result<(), anyhow::Error> {
        let _: String = self
            .post_public_api(&format!("agent/tasks/{task_id}/cancel"), &())
            .await?;
        Ok(())
    }

    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error> {
        let auth_token = self.get_or_refresh_access_token().await?;
        let request_builder = self.client.post(format!(
            "{}/ai/generate_code_review_content",
            ChannelState::server_root_url()
        ));
        let response = if let Some(token) = auth_token.as_bearer_token() {
            request_builder.bearer_auth(token)
        } else {
            request_builder
        }
        .json(&request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response)
    }
}


#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
