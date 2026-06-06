use std::collections::{HashMap, HashSet};
use std::time::Duration;

use ai::index::full_source_code_embedding::store_client::{IntermediateNode, StoreClient};
use ai::index::full_source_code_embedding::{
    self, CodebaseContextConfig, ContentHash, EmbeddingConfig, NodeHash, RepoMetadata,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cynic::{MutationBuilder, QueryBuilder};
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use warp_core::channel::ChannelState;
use warp_core::report_error;
use warp_graphql::mutations::create_agent_task::{
    CreateAgentTask, CreateAgentTaskInput, CreateAgentTaskResult, CreateAgentTaskVariables,
};
use warp_graphql::mutations::generate_code_embeddings::{
    GenerateCodeEmbeddings, GenerateCodeEmbeddingsInput, GenerateCodeEmbeddingsResult,
    GenerateCodeEmbeddingsVariables,
};
use warp_graphql::mutations::generate_metadata_for_command::{
    GenerateMetadataForCommand, GenerateMetadataForCommandInput, GenerateMetadataForCommandResult,
    GenerateMetadataForCommandStatus, GenerateMetadataForCommandVariables,
};
use warp_graphql::mutations::populate_merkle_tree_cache::{
    PopulateMerkleTreeCache, PopulateMerkleTreeCacheResult, PopulateMerkleTreeCacheVariables,
};
use warp_graphql::mutations::update_merkle_tree::{
    MerkleTreeNode, UpdateMerkleTree, UpdateMerkleTreeInput, UpdateMerkleTreeResult,
    UpdateMerkleTreeVariables,
};
use warp_graphql::queries::codebase_context_config::{
    CodebaseContextConfigQuery, CodebaseContextConfigResult, CodebaseContextConfigVariables,
};
use warp_graphql::queries::get_available_harnesses::{
    GetAvailableHarnesses, GetAvailableHarnessesVariables,
};
use warp_graphql::queries::get_feature_model_choices::{
    GetFeatureModelChoices, GetFeatureModelChoicesVariables,
};
use warp_graphql::queries::get_relevant_fragments::{
    GetRelevantFragmentsQuery, GetRelevantFragmentsResult, GetRelevantFragmentsVariables,
};
#[cfg(not(feature = "agent_mode_evals"))]
use warp_graphql::queries::get_request_limit_info::{
    GetRequestLimitInfo, GetRequestLimitInfoVariables,
};
use warp_graphql::queries::rerank_fragments::{
    RerankFragments, RerankFragmentsResult, RerankFragmentsVariables,
};
use warp_graphql::queries::sync_merkle_tree::{
    SyncMerkleTree, SyncMerkleTreeInput, SyncMerkleTreeResult, SyncMerkleTreeVariables,
};
use super::auth::AuthClient;
use super::ServerApi;
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::{AIAgentHarness, ServerAIConversationMetadata};
use crate::ai::ambient_agents::AmbientAgentTaskId;
// Re-export ambient agent types for backwards compatibility
pub use crate::ai::ambient_agents::{AgentConfigSnapshot, AgentSource};
use crate::ai::artifacts::Artifact;
use crate::ai::generate_code_review_content::api::{
    GenerateCodeReviewContentRequest, GenerateCodeReviewContentResponse,
};
use crate::ai::harness_availability::HarnessAvailability;
use crate::ai::llms::{
    AvailableLLMs, DisableReason, LLMContextWindow, LLMInfo, LLMModelHost, LLMProvider, LLMSpec,
    LLMUsageMetadata, ModelsByFeature, RoutingHostConfig,
};
#[cfg(feature = "agent_mode_evals")]
use crate::ai::request_usage_model::RequestLimitInfo;
#[cfg(not(feature = "agent_mode_evals"))]
use crate::ai::BonusGrant;
use crate::ai::RequestUsageInfo;
use crate::drive::workflows::ai_assist::{GeneratedCommandMetadata, GeneratedCommandMetadataError};
use crate::persistence::model::ConversationUsageMetadata;
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

    async fn get_feature_model_choices(&self) -> Result<ModelsByFeature, anyhow::Error>;

    async fn get_available_harnesses(&self) -> Result<Vec<HarnessAvailability>, anyhow::Error>;
    async fn list_connected_self_hosted_workers(
        &self,
    ) -> Result<ListConnectedSelfHostedWorkersResponse, anyhow::Error>;

    async fn update_merkle_tree(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> anyhow::Result<HashMap<NodeHash, bool>>;

    async fn generate_code_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> anyhow::Result<HashMap<ContentHash, bool>>;


    async fn create_agent_task(
        &self,
        prompt: String,
        environment_uid: Option<String>,
        parent_run_id: Option<String>,
        config: Option<AgentConfigSnapshot>,
    ) -> anyhow::Result<AmbientAgentTaskId, anyhow::Error>;

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

    async fn get_feature_model_choices(&self) -> Result<ModelsByFeature, anyhow::Error> {
        let variables = GetFeatureModelChoicesVariables {
            request_context: get_request_context(),
        };
        let operation = GetFeatureModelChoices::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_feature_model_choices::UserResult::UserOutput(
                warp_graphql::queries::get_feature_model_choices::UserOutput {
                    user: warp_graphql::queries::get_feature_model_choices::User { mut workspaces },
                },
            ) if !workspaces.is_empty() => {
                // This is safe (`remove()` can panic) because we ensure workspaces is non-empty
                // above.
                workspaces.remove(0).feature_model_choice.try_into()
            }
            _ => Err(anyhow!("Failed to get available feature model choices")),
        }
    }

    async fn get_available_harnesses(&self) -> Result<Vec<HarnessAvailability>, anyhow::Error> {
        let variables = GetAvailableHarnessesVariables {
            request_context: get_request_context(),
        };
        let operation = GetAvailableHarnesses::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.user {
            warp_graphql::queries::get_available_harnesses::UserResult::UserOutput(output) => {
                Ok(output
                    .user
                    .available_harnesses
                    .harnesses
                    .into_iter()
                    .map(|h| HarnessAvailability {
                        harness: convert_harness(h.harness).into(),
                        display_name: h.display_name,
                        enabled: h.enabled,
                        available_models: h
                            .available_models
                            .into_iter()
                            .map(|m| crate::ai::harness_availability::HarnessModelInfo {
                                id: m.id.into_inner(),
                                display_name: m.display_name,
                                reasoning_level: m.reasoning_level,
                            })
                            .collect(),
                    })
                    .collect())
            }
            warp_graphql::queries::get_available_harnesses::UserResult::Unknown => {
                Err(anyhow!("Failed to get available harnesses"))
            }
        }
    }

    async fn update_merkle_tree(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> anyhow::Result<HashMap<NodeHash, bool>> {
        let nodes = nodes
            .into_iter()
            .map(|node| MerkleTreeNode {
                hash: node.hash.into(),
                children: node.children.into_iter().map(Into::into).collect(),
            })
            .collect_vec();
        let variables = UpdateMerkleTreeVariables {
            input: UpdateMerkleTreeInput {
                embedding_config: embedding_config.into(),
                nodes,
            },
            request_context: get_request_context(),
        };
        let operation = UpdateMerkleTree::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.update_merkle_tree {
            UpdateMerkleTreeResult::UpdateMerkleTreeOutput(output) => {
                let mut node_results = HashMap::with_capacity(output.results.len());
                for result in output.results {
                    node_results.insert(result.hash.try_into()?, result.success);
                }
                Ok(node_results)
            }
            UpdateMerkleTreeResult::UpdateMerkleTreeError(e) => Err(anyhow!(e.error)),
            UpdateMerkleTreeResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            UpdateMerkleTreeResult::Unknown => Err(anyhow!("failed to update merkle tree")),
        }
    }

    async fn generate_code_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> anyhow::Result<HashMap<ContentHash, bool>> {
        let variables = GenerateCodeEmbeddingsVariables {
            input: GenerateCodeEmbeddingsInput {
                embedding_config: embedding_config.into(),
                fragments: fragments.into_iter().map(Into::into).collect(),
                repo_metadata: repo_metadata.into(),
                root_hash: root_hash.into(),
            },
            request_context: get_request_context(),
        };

        let operation = GenerateCodeEmbeddings::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.generate_code_embeddings {
            GenerateCodeEmbeddingsResult::GenerateCodeEmbeddingsOutput(output) => {
                let mut results = HashMap::with_capacity(output.embedding_results.len());
                for result in output.embedding_results {
                    results.insert(result.hash.try_into()?, result.success);
                }
                Ok(results)
            }
            GenerateCodeEmbeddingsResult::GenerateCodeEmbeddingsError(e) => Err(anyhow!(e.error)),
            GenerateCodeEmbeddingsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            GenerateCodeEmbeddingsResult::Unknown => {
                Err(anyhow!("failed to generate code embeddings"))
            }
        }
    }

    async fn create_agent_task(
        &self,
        prompt: String,
        environment_uid: Option<String>,
        parent_run_id: Option<String>,
        config: Option<AgentConfigSnapshot>,
    ) -> anyhow::Result<AmbientAgentTaskId, anyhow::Error> {
        // Serialize the config to JSON if provided
        let agent_config_snapshot = config
            .map(|c| serde_json::to_string(&c))
            .transpose()
            .map_err(|e| anyhow!("Failed to serialize agent config: {e}"))?;

        let variables = CreateAgentTaskVariables {
            input: CreateAgentTaskInput {
                prompt,
                environment_uid: environment_uid.map(|uid| uid.into()),
                parent_run_id: parent_run_id.map(|run_id| run_id.into()),
                agent_config_snapshot,
            },
            request_context: get_request_context(),
        };

        let operation = CreateAgentTask::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.create_agent_task {
            CreateAgentTaskResult::CreateAgentTaskOutput(output) => output
                .task_id
                .into_inner()
                .parse()
                .map_err(|e| anyhow!("Failed to parse task ID from server: {e}")),
            CreateAgentTaskResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)))
            }
            CreateAgentTaskResult::Unknown => Err(anyhow!("failed to create agent task")),
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

impl TryFrom<warp_graphql::queries::get_feature_model_choices::FeatureModelChoice>
    for ModelsByFeature
{
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::get_feature_model_choices::FeatureModelChoice,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            agent_mode: value.agent_mode.try_into()?,
            coding: value.coding.try_into()?,
            cli_agent: Some(value.cli_agent.try_into()?),
            computer_use: Some(value.computer_use_agent.try_into()?),
        })
    }
}

impl TryFrom<warp_graphql::workspace::FeatureModelChoice> for ModelsByFeature {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::workspace::FeatureModelChoice) -> Result<Self, Self::Error> {
        Ok(Self {
            agent_mode: value.agent_mode.try_into()?,
            coding: value.coding.try_into()?,
            cli_agent: Some(value.cli_agent.try_into()?),
            computer_use: Some(value.computer_use_agent.try_into()?),
        })
    }
}

impl TryFrom<warp_graphql::queries::get_feature_model_choices::AvailableLlms> for AvailableLLMs {
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::get_feature_model_choices::AvailableLlms,
    ) -> Result<Self, Self::Error> {
        Self::new(
            value.default_id.into(),
            value.choices.into_iter().map(LLMInfo::from),
            value.preferred_codex_model_id.map(Into::into),
        )
    }
}

impl TryFrom<warp_graphql::workspace::AvailableLlms> for AvailableLLMs {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::workspace::AvailableLlms) -> Result<Self, Self::Error> {
        Self::new(
            value.default_id.into(),
            value.choices.into_iter().map(LLMInfo::from),
            value.preferred_codex_model_id.map(Into::into),
        )
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmInfo> for LLMInfo {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmInfo) -> Self {
        let host_configs = {
            let mut map = std::collections::HashMap::new();
            for config in value.host_configs {
                let config: RoutingHostConfig = config.into();
                let host = config.model_routing_host.clone();
                if map.insert(host.clone(), config).is_some() {
                    log::warn!(
                        "Duplicate LlmModelHost entry for {:?}, using latest value",
                        host
                    );
                }
            }
            map
        };
        Self {
            id: value.id.into(),
            display_name: value.display_name,
            base_model_name: value.base_model_name,
            reasoning_level: value.reasoning_level,
            usage_metadata: value.usage_metadata.into(),
            description: value.description,
            disable_reason: value.disable_reason.map(DisableReason::from),
            vision_supported: value.vision_supported,
            spec: value.spec.map(Into::into),
            provider: value.provider.into(),
            host_configs,
            discount_percentage: value.pricing.discount_percentage.map(|v| v as f32),
            context_window: LLMContextWindow {
                is_configurable: value.context_window.is_configurable,
                min: value.context_window.min.into(),
                max: value.context_window.max.into(),
                default_max: value.context_window.default.into(),
            },
        }
    }
}

impl From<warp_graphql::workspace::LlmInfo> for LLMInfo {
    fn from(value: warp_graphql::workspace::LlmInfo) -> Self {
        let host_configs = {
            let mut map = std::collections::HashMap::new();
            for config in value.host_configs {
                let config: RoutingHostConfig = config.into();
                let host = config.model_routing_host.clone();
                if map.insert(host.clone(), config).is_some() {
                    log::warn!(
                        "Duplicate LlmModelHost entry for {:?}, using latest value",
                        host
                    );
                }
            }
            map
        };
        Self {
            id: value.id.into(),
            display_name: value.display_name,
            base_model_name: value.base_model_name,
            reasoning_level: value.reasoning_level,
            usage_metadata: value.usage_metadata.into(),
            description: value.description,
            disable_reason: value.disable_reason.map(DisableReason::from),
            vision_supported: value.vision_supported,
            spec: value.spec.map(Into::into),
            provider: value.provider.into(),
            host_configs,
            discount_percentage: value.pricing.discount_percentage.map(|v| v as f32),
            context_window: LLMContextWindow {
                is_configurable: value.context_window.is_configurable,
                min: value.context_window.min.into(),
                max: value.context_window.max.into(),
                default_max: value.context_window.default.into(),
            },
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::RoutingHostConfig>
    for RoutingHostConfig
{
    fn from(value: warp_graphql::queries::get_feature_model_choices::RoutingHostConfig) -> Self {
        Self {
            enabled: value.enabled,
            model_routing_host: value.model_routing_host.into(),
        }
    }
}

impl From<warp_graphql::workspace::RoutingHostConfig> for RoutingHostConfig {
    fn from(value: warp_graphql::workspace::RoutingHostConfig) -> Self {
        Self {
            enabled: value.enabled,
            model_routing_host: value.model_routing_host.into(),
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmModelHost> for LLMModelHost {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmModelHost) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::DirectApi => {
                LLMModelHost::DirectApi
            }
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::AwsBedrock => {
                LLMModelHost::AwsBedrock
            }
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::CustomEndpoint => {
                LLMModelHost::CustomEndpoint
            }
            warp_graphql::queries::get_feature_model_choices::LlmModelHost::Other(value) => {
                report_error!(anyhow!(
                    "Unknown LlmModelHost '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMModelHost::Unknown
            }
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmProvider> for LLMProvider {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmProvider) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Openai => {
                LLMProvider::OpenAI
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Anthropic => {
                LLMProvider::Anthropic
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Google => {
                LLMProvider::Google
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Xai => LLMProvider::Xai,
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Unknown => {
                LLMProvider::Unknown
            }
            warp_graphql::queries::get_feature_model_choices::LlmProvider::Other(value) => {
                report_error!(anyhow!(
                    "Invalid LlmProvider '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMProvider::Unknown
            }
        }
    }
}

impl From<warp_graphql::workspace::LlmProvider> for LLMProvider {
    fn from(value: warp_graphql::workspace::LlmProvider) -> Self {
        match value {
            warp_graphql::workspace::LlmProvider::Openai => LLMProvider::OpenAI,
            warp_graphql::workspace::LlmProvider::Anthropic => LLMProvider::Anthropic,
            warp_graphql::workspace::LlmProvider::Google => LLMProvider::Google,
            warp_graphql::workspace::LlmProvider::Xai => LLMProvider::Xai,
            warp_graphql::workspace::LlmProvider::Unknown => LLMProvider::Unknown,
            warp_graphql::workspace::LlmProvider::Other(value) => {
                report_error!(anyhow!(
                    "Invalid LlmProvider '{value}'. Make sure to update client GraphQL types!"
                ));
                LLMProvider::Unknown
            }
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmSpec> for LLMSpec {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmSpec) -> Self {
        Self {
            cost: value.cost as f32,
            quality: value.quality as f32,
            speed: value.speed as f32,
        }
    }
}

impl From<warp_graphql::workspace::LlmSpec> for LLMSpec {
    fn from(value: warp_graphql::workspace::LlmSpec) -> Self {
        Self {
            cost: value.cost as f32,
            quality: value.quality as f32,
            speed: value.speed as f32,
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::LlmUsageMetadata> for LLMUsageMetadata {
    fn from(value: warp_graphql::queries::get_feature_model_choices::LlmUsageMetadata) -> Self {
        Self {
            request_multiplier: value.request_multiplier.max(1) as usize,
            credit_multiplier: value.credit_multiplier.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::workspace::LlmUsageMetadata> for LLMUsageMetadata {
    fn from(value: warp_graphql::workspace::LlmUsageMetadata) -> Self {
        Self {
            request_multiplier: value.request_multiplier.max(1) as usize,
            credit_multiplier: value.credit_multiplier.map(|v| v as f32),
        }
    }
}

impl From<warp_graphql::queries::get_feature_model_choices::DisableReason> for DisableReason {
    fn from(value: warp_graphql::queries::get_feature_model_choices::DisableReason) -> Self {
        match value {
            warp_graphql::queries::get_feature_model_choices::DisableReason::AdminDisabled => {
                DisableReason::AdminDisabled
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::OutOfRequests => {
                DisableReason::OutOfRequests
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::ProviderOutage => {
                DisableReason::ProviderOutage
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::RequiresUpgrade => {
                DisableReason::RequiresUpgrade
            }
            warp_graphql::queries::get_feature_model_choices::DisableReason::Other(_) => {
                DisableReason::Unavailable
            }
        }
    }
}

impl From<warp_graphql::workspace::DisableReason> for DisableReason {
    fn from(value: warp_graphql::workspace::DisableReason) -> Self {
        match value {
            warp_graphql::workspace::DisableReason::AdminDisabled => DisableReason::AdminDisabled,
            warp_graphql::workspace::DisableReason::OutOfRequests => DisableReason::OutOfRequests,
            warp_graphql::workspace::DisableReason::ProviderOutage => DisableReason::ProviderOutage,
            warp_graphql::workspace::DisableReason::RequiresUpgrade => {
                DisableReason::RequiresUpgrade
            }
            warp_graphql::workspace::DisableReason::Other(_) => DisableReason::Unavailable,
        }
    }
}

// Conversions for AIConversationMetadata from GraphQL types

fn convert_harness(harness: warp_graphql::ai::AgentHarness) -> AIAgentHarness {
    match harness {
        warp_graphql::ai::AgentHarness::Oz => AIAgentHarness::Oz,
        warp_graphql::ai::AgentHarness::ClaudeCode => AIAgentHarness::ClaudeCode,
        warp_graphql::ai::AgentHarness::Gemini => AIAgentHarness::Gemini,
        warp_graphql::ai::AgentHarness::Codex => AIAgentHarness::Codex,
        warp_graphql::ai::AgentHarness::Other(value) => {
            report_error!(anyhow!(
                "Invalid AgentHarness '{value}'. Make sure to update client GraphQL types!"
            ));
            AIAgentHarness::Unknown
        }
    }
}

fn convert_usage_metadata(
    summarized: bool,
    context_window_usage: f64,
    credits_spent: f64,
) -> ConversationUsageMetadata {
    ConversationUsageMetadata {
        was_summarized: summarized,
        context_window_usage: context_window_usage as f32,
        credits_spent: credits_spent as f32,
        credits_spent_for_last_block: None,
        token_usage: vec![],
        tool_usage_metadata: Default::default(),
    }
}

impl TryFrom<warp_graphql::ai::AIConversation> for ServerAIConversationMetadata {
    type Error = anyhow::Error;

    fn try_from(value: warp_graphql::ai::AIConversation) -> Result<Self, Self::Error> {
        let usage = convert_usage_metadata(
            value.usage.usage_metadata.summarized,
            value.usage.usage_metadata.context_window_usage,
            value.usage.usage_metadata.credits_spent,
        );
        let metadata = value.metadata.try_into()?;
        let permissions = value.permissions.try_into()?;
        let ambient_agent_task_id = value
            .ambient_agent_task_id
            .map(|id| id.into_inner().parse())
            .transpose()?;
        let server_conversation_token =
            ServerConversationToken::new(value.conversation_id.into_inner());

        // If we fail to parse any artifacts, don't fail the entire conversion -- just don't include them in the list
        let artifacts = value
            .artifacts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| Artifact::try_from(a).ok())
            .collect();

        Ok(Self {
            title: value.title,
            working_directory: value.working_directory,
            harness: convert_harness(value.harness),
            usage,
            metadata,
            permissions,
            ambient_agent_task_id,
            server_conversation_token,
            artifacts,
        })
    }
}

impl TryFrom<warp_graphql::queries::list_ai_conversations::AIConversationMetadata>
    for ServerAIConversationMetadata
{
    type Error = anyhow::Error;

    fn try_from(
        value: warp_graphql::queries::list_ai_conversations::AIConversationMetadata,
    ) -> Result<Self, Self::Error> {
        let usage = convert_usage_metadata(
            value.usage.usage_metadata.summarized,
            value.usage.usage_metadata.context_window_usage,
            value.usage.usage_metadata.credits_spent,
        );
        let metadata = value.metadata.try_into()?;
        let permissions = value.permissions.try_into()?;
        let ambient_agent_task_id = value
            .ambient_agent_task_id
            .map(|id| id.into_inner().parse())
            .transpose()?;
        let server_conversation_token =
            ServerConversationToken::new(value.conversation_id.into_inner());

        let artifacts = value
            .artifacts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| Artifact::try_from(a).ok())
            .collect();

        Ok(Self {
            title: value.title,
            working_directory: value.working_directory,
            harness: convert_harness(value.harness),
            usage,
            metadata,
            permissions,
            ambient_agent_task_id,
            server_conversation_token,
            artifacts,
        })
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl StoreClient for ServerApi {
    async fn update_intermediate_nodes(
        &self,
        embedding_config: EmbeddingConfig,
        nodes: Vec<IntermediateNode>,
    ) -> Result<HashMap<NodeHash, bool>, full_source_code_embedding::Error> {
        let results = self.update_merkle_tree(embedding_config, nodes).await?;
        Ok(results)
    }

    async fn generate_embeddings(
        &self,
        embedding_config: EmbeddingConfig,
        fragments: Vec<full_source_code_embedding::Fragment>,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<HashMap<ContentHash, bool>, full_source_code_embedding::Error> {
        let results = self
            .generate_code_embeddings(embedding_config, fragments, root_hash, repo_metadata)
            .await?;
        Ok(results)
    }

    async fn populate_merkle_tree_cache(
        &self,
        embedding_config: EmbeddingConfig,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<bool, full_source_code_embedding::Error> {
        let variables = PopulateMerkleTreeCacheVariables {
            embedding_config: embedding_config.into(),
            root_hash: root_hash.into(),
            repo_metadata: repo_metadata.into(),
            request_context: get_request_context(),
        };
        let operation = PopulateMerkleTreeCache::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.populate_merkle_tree_cache {
            PopulateMerkleTreeCacheResult::PopulateMerkleTreeCacheOutput(output) => {
                Ok(output.success)
            }
            PopulateMerkleTreeCacheResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            PopulateMerkleTreeCacheResult::Unknown => {
                Err(anyhow!("failed to populate merkle tree cache").into())
            }
        }
    }

    async fn sync_merkle_tree(
        &self,
        nodes: Vec<NodeHash>,
        embedding_config: EmbeddingConfig,
    ) -> Result<HashSet<NodeHash>, full_source_code_embedding::Error> {
        let input = SyncMerkleTreeInput {
            hashed_nodes: nodes.into_iter().map(Into::into).collect(),
            embedding_config: embedding_config.into(),
        };

        let variables = SyncMerkleTreeVariables {
            input,
            request_context: get_request_context(),
        };

        let operation = SyncMerkleTree::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.sync_merkle_tree {
            SyncMerkleTreeResult::SyncMerkleTreeOutput(output) => {
                let mut node_results = HashSet::with_capacity(output.changed_nodes.len());
                for hash in output.changed_nodes {
                    node_results.insert(hash.try_into()?);
                }
                Ok(node_results)
            }
            SyncMerkleTreeResult::SyncMerkleTreeError(e) => Err(anyhow!(e.error).into()),
            SyncMerkleTreeResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            SyncMerkleTreeResult::Unknown => Err(anyhow!("failed to sync merkle tree").into()),
        }
    }

    async fn rerank_fragments(
        &self,
        query: String,
        fragments: Vec<full_source_code_embedding::Fragment>,
    ) -> Result<Vec<full_source_code_embedding::Fragment>, full_source_code_embedding::Error> {
        let variables = RerankFragmentsVariables {
            query,
            fragments: fragments.into_iter().map(Into::into).collect(),
            request_context: get_request_context(),
        };
        let operation = RerankFragments::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.rerank_fragments {
            RerankFragmentsResult::RerankFragmentsOutput(output) => Ok(output
                .ranked_fragments
                .into_iter()
                .map(|fragment| fragment.try_into())
                .collect::<Result<Vec<_>, _>>()?),
            RerankFragmentsResult::RerankFragmentsError(e) => Err(anyhow!(e.error).into()),
            RerankFragmentsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            RerankFragmentsResult::Unknown => Err(anyhow!("failed to rerank fragments").into()),
        }
    }

    async fn get_relevant_fragments(
        &self,
        embedding_config: EmbeddingConfig,
        query: String,
        root_hash: NodeHash,
        repo_metadata: RepoMetadata,
    ) -> Result<Vec<ContentHash>, full_source_code_embedding::Error> {
        let variables = GetRelevantFragmentsVariables {
            query,
            root_hash: root_hash.into(),
            embedding_config: embedding_config.into(),
            request_context: get_request_context(),
            repo_metadata: repo_metadata.into(),
        };
        let operation = GetRelevantFragmentsQuery::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.get_relevant_fragments {
            GetRelevantFragmentsResult::GetRelevantFragmentsOutput(output) => Ok(output
                .candidate_hashes
                .into_iter()
                .map(|hash| hash.try_into())
                .collect::<Result<Vec<_>, _>>()?),
            GetRelevantFragmentsResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            GetRelevantFragmentsResult::GetRelevantFragmentsError(e) => {
                Err(anyhow!(e.error).into())
            }
            GetRelevantFragmentsResult::Unknown => {
                Err(anyhow!("failed to get relevant fragments").into())
            }
        }
    }

    async fn codebase_context_config(
        &self,
    ) -> Result<CodebaseContextConfig, full_source_code_embedding::Error> {
        let variables = CodebaseContextConfigVariables {
            request_context: get_request_context(),
        };
        let operation = CodebaseContextConfigQuery::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.codebase_context_config {
            CodebaseContextConfigResult::CodebaseContextConfigOutput(output) => {
                Ok(CodebaseContextConfig {
                    embedding_config: output.embedding_config.try_into()?,
                    embedding_cadence: Duration::from_secs(output.embedding_cadence as u64),
                })
            }
            CodebaseContextConfigResult::UserFacingError(e) => {
                Err(anyhow!(get_user_facing_error_message(e)).into())
            }
            CodebaseContextConfigResult::Unknown => {
                Err(anyhow!("failed to retrieve codebase context config").into())
            }
        }
    }
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
