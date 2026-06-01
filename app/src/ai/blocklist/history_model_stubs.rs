use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use warpui::{AppContext, Entity, EntityId, ModelContext, SingletonEntity};

use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::{
    AIConversation, AIConversationId, ConversationStatus, ServerAIConversationMetadata,
};
use crate::ai::agent::task::TaskId;
use crate::ai::agent::{
    AIAgentActionId, AIAgentExchange, AIAgentExchangeId, AIAgentOutputStatus,
    FinishedAIAgentOutput, Suggestions,
};
use crate::ai::artifacts::Artifact;
use crate::persistence::model::{AgentConversation, AgentConversationData};
use crate::server::server_api::ai::AIClient;
use crate::terminal::model::block::{BlockId, SerializedBlock};
use crate::ui_components::icons::Icon;

use super::ResponseStreamId;
use super::action_stubs::StartAgentRequestId;

pub const FORK_PREFIX: &str = "(Fork) ";
pub const PRE_REWIND_PREFIX: &str = "(Pre-Rewind) ";

// --- Conversation loader stubs ---

#[derive(Debug, Clone)]
pub struct CLIAgentConversation {
    pub metadata: ServerAIConversationMetadata,
    pub block: SerializedBlock,
}

pub enum CloudConversationData {
    Oz(Box<AIConversation>),
    CLIAgent(Box<CLIAgentConversation>),
}

pub fn convert_persisted_conversation_to_ai_conversation_with_metadata(
    _persisted_conversation: AgentConversation,
) -> Option<AIConversation> {
    None
}

pub async fn load_conversation_from_server(
    _conversation_id: AIConversationId,
    _server_conversation_token: ServerConversationToken,
    _server_api: Arc<dyn AIClient>,
) -> Option<CloudConversationData> {
    None
}

// --- Data types ---

#[derive(Debug, Clone)]
pub struct AIConversationMetadata {
    pub id: AIConversationId,
    pub title: String,
    pub initial_query: String,
    pub last_modified_at: NaiveDateTime,
    pub initial_working_directory: Option<String>,
    pub credits_spent: Option<f32>,
    pub server_conversation_token: Option<ServerConversationToken>,
    pub has_local_data: bool,
    pub has_cloud_data: bool,
    pub artifacts: Vec<Artifact>,
    pub server_conversation_metadata: Option<ServerAIConversationMetadata>,
}

impl From<&AIConversation> for AIConversationMetadata {
    fn from(conversation: &AIConversation) -> Self {
        Self {
            id: conversation.id(),
            title: conversation.title().unwrap_or_default().to_string(),
            initial_query: String::new(),
            last_modified_at: chrono::Utc::now().naive_utc(),
            initial_working_directory: None,
            credits_spent: None,
            server_conversation_token: None,
            has_local_data: true,
            has_cloud_data: false,
            artifacts: vec![],
            server_conversation_metadata: None,
        }
    }
}

impl AIConversationMetadata {
    pub fn from_server_metadata(
        conversation_id: AIConversationId,
        server_conversation_metadata: ServerAIConversationMetadata,
    ) -> Self {
        Self {
            id: conversation_id,
            title: server_conversation_metadata.title.clone(),
            initial_query: String::new(),
            last_modified_at: chrono::Utc::now().naive_utc(),
            initial_working_directory: server_conversation_metadata.working_directory.clone(),
            credits_spent: Some(server_conversation_metadata.usage.credits_spent),
            server_conversation_token: Some(server_conversation_metadata.server_conversation_token.clone()),
            has_local_data: false,
            has_cloud_data: true,
            artifacts: server_conversation_metadata.artifacts.clone(),
            server_conversation_metadata: Some(server_conversation_metadata),
        }
    }

    pub fn is_ambient_agent_conversation(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateHistoryError {
    ConversationNotFound(AIConversationId),
    ExchangeNotFound,
    TaskNotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationStatusUpdate {
    Restored,
    Changed { prev_status: ConversationStatus },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIQueryHistoryOutputStatus {
    Pending,
    Cancelled,
    Failed,
    Completed,
}

impl AIQueryHistoryOutputStatus {
    pub(crate) fn display_text(&self) -> &'static str {
        match self {
            Self::Completed => "Completed successfully",
            Self::Pending => "Pending",
            Self::Cancelled => "Cancelled by user",
            Self::Failed => "Failed",
        }
    }

    pub(crate) fn icon(&self) -> Icon {
        match self {
            Self::Completed => Icon::Check,
            Self::Pending => Icon::Loading,
            Self::Cancelled => Icon::SlashCircle,
            Self::Failed => Icon::AlertTriangle,
        }
    }
}

impl From<&AIAgentOutputStatus> for AIQueryHistoryOutputStatus {
    fn from(status: &AIAgentOutputStatus) -> Self {
        match status {
            AIAgentOutputStatus::Streaming { .. } => Self::Pending,
            AIAgentOutputStatus::Finished { finished_output, .. } => match finished_output {
                FinishedAIAgentOutput::Cancelled { .. } => Self::Cancelled,
                FinishedAIAgentOutput::Error { .. } => Self::Failed,
                FinishedAIAgentOutput::Success { .. } => Self::Completed,
            },
        }
    }
}

use crate::input_suggestions::HistoryOrder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AIQueryHistory {
    pub query_text: String,
    pub start_time: DateTime<Local>,
    pub output_status: AIQueryHistoryOutputStatus,
    pub working_directory: Option<String>,
    pub history_order: HistoryOrder,
}

#[cfg(any(test, feature = "integration_tests"))]
impl AIQueryHistory {
    pub fn new_for_test(
        query_text: impl Into<String>,
        start_time: impl Into<chrono::DateTime<chrono::Local>>,
        history_order: crate::input_suggestions::HistoryOrder,
    ) -> Self {
        Self {
            query_text: query_text.into(),
            start_time: start_time.into(),
            output_status: AIQueryHistoryOutputStatus::Completed,
            working_directory: None,
            history_order,
        }
    }
}

// --- Event enum ---

#[derive(Clone, Debug)]
pub enum BlocklistAIHistoryEvent {
    StartedNewConversation {
        new_conversation_id: AIConversationId,
        terminal_view_id: EntityId,
    },
    CreatedSubtask {
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
        task_id: TaskId,
    },
    UpgradedTask {
        optimistic_id: TaskId,
        server_id: TaskId,
        terminal_view_id: EntityId,
    },
    AppendedExchange {
        exchange_id: AIAgentExchangeId,
        task_id: TaskId,
        terminal_view_id: EntityId,
        conversation_id: AIConversationId,
        is_hidden: bool,
        response_stream_id: Option<ResponseStreamId>,
    },
    ReassignedExchange {
        exchange_id: AIAgentExchangeId,
        terminal_view_id: EntityId,
        new_task_id: TaskId,
        new_conversation_id: AIConversationId,
    },
    #[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
    UpdatedStreamingExchange {
        exchange_id: AIAgentExchangeId,
        terminal_view_id: EntityId,
        conversation_id: AIConversationId,
        is_hidden: bool,
    },
    UpdatedConversationStatus {
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
        update: ConversationStatusUpdate,
        new_status: ConversationStatus,
    },
    SetActiveConversation {
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
    },
    ClearedActiveConversation {
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
    },
    ClearedConversationsInTerminalView {
        terminal_view_id: EntityId,
        active_conversation_id: Option<AIConversationId>,
    },
    UpdatedTodoList {
        terminal_view_id: EntityId,
    },
    UpdatedAutoexecuteOverride {
        terminal_view_id: EntityId,
    },
    SplitConversation {
        terminal_view_id: EntityId,
        old_conversation_id: AIConversationId,
        new_conversation_id: AIConversationId,
    },
    RemoveConversation {
        terminal_view_id: EntityId,
        conversation_id: AIConversationId,
        run_id: Option<String>,
    },
    DeletedConversation {
        terminal_view_id: EntityId,
        conversation_id: AIConversationId,
        conversation_title: Option<String>,
        run_id: Option<String>,
    },
    RestoredConversations {
        terminal_view_id: EntityId,
        conversation_ids: Vec<AIConversationId>,
    },
    UpdatedConversationMetadata {
        terminal_view_id: Option<EntityId>,
        conversation_id: AIConversationId,
    },
    UpdatedConversationArtifacts {
        terminal_view_id: EntityId,
        conversation_id: AIConversationId,
        artifact: Artifact,
    },
    ConversationServerTokenAssigned {
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
    },
    ConversationOwnershipTransferred {
        conversation_id: AIConversationId,
        previous_terminal_view_id: EntityId,
        new_terminal_view_id: EntityId,
    },
    NewConversationRequestComplete {
        request_id: StartAgentRequestId,
        conversation_id: AIConversationId,
    },
    OrchestrationConfigUpdated {
        conversation_id: AIConversationId,
        from_restore: bool,
    },
    ConversationUsageMetadataUpdated {
        conversation_id: AIConversationId,
    },
    LocalSharedSessionEstablished {
        conversation_id: AIConversationId,
        session_id: session_sharing_protocol::common::SessionId,
    },
}

impl BlocklistAIHistoryEvent {
    pub fn terminal_view_id(&self) -> Option<EntityId> {
        match self {
            BlocklistAIHistoryEvent::StartedNewConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::AppendedExchange { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpdatedStreamingExchange { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpdatedConversationStatus { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::SetActiveConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::ClearedActiveConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::ClearedConversationsInTerminalView { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::ReassignedExchange { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpdatedTodoList { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpdatedAutoexecuteOverride { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::SplitConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::RemoveConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::DeletedConversation { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::CreatedSubtask { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::RestoredConversations { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpgradedTask { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::ConversationOwnershipTransferred { previous_terminal_view_id: terminal_view_id, .. }
            | BlocklistAIHistoryEvent::UpdatedConversationArtifacts { terminal_view_id, .. }
            | BlocklistAIHistoryEvent::ConversationServerTokenAssigned { terminal_view_id, .. } => Some(*terminal_view_id),
            BlocklistAIHistoryEvent::UpdatedConversationMetadata { terminal_view_id, .. } => *terminal_view_id,
            BlocklistAIHistoryEvent::NewConversationRequestComplete { .. }
            | BlocklistAIHistoryEvent::OrchestrationConfigUpdated { .. }
            | BlocklistAIHistoryEvent::ConversationUsageMetadataUpdated { .. }
            | BlocklistAIHistoryEvent::LocalSharedSessionEstablished { .. } => None,
        }
    }
}

// --- Main model ---

#[derive(Default)]
pub struct BlocklistAIHistoryModel {
    conversations_by_id: HashMap<AIConversationId, AIConversation>,
    children_by_parent: HashMap<AIConversationId, Vec<AIConversationId>>,
    empty_conversation_ids: Vec<AIConversationId>,
}

impl Entity for BlocklistAIHistoryModel {
    type Event = BlocklistAIHistoryEvent;
}

impl SingletonEntity for BlocklistAIHistoryModel {}

impl BlocklistAIHistoryModel {
    pub(crate) fn new(
        _persisted_queries: Vec<super::persistence::PersistedAIInput>,
        _multi_agent_conversations: &[crate::persistence::model::AgentConversation],
    ) -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        Self::default()
    }

    pub fn conversation(&self, conversation_id: &AIConversationId) -> Option<&AIConversation> {
        self.conversations_by_id.get(conversation_id)
    }

    pub fn conversation_mut(&mut self, conversation_id: &AIConversationId) -> Option<&mut AIConversation> {
        self.conversations_by_id.get_mut(conversation_id)
    }

    pub fn active_conversation(&self, _terminal_view_id: EntityId) -> Option<&AIConversation> {
        None
    }

    pub(crate) fn active_conversation_id(&self, _terminal_view_id: EntityId) -> Option<AIConversationId> {
        None
    }

    pub(crate) fn last_conversation_id(&self, _terminal_view_id: EntityId) -> Option<AIConversationId> {
        None
    }

    pub fn terminal_view_id_for_conversation(&self, _conversation_id: &AIConversationId) -> Option<EntityId> {
        None
    }

    pub fn conversation_status(&self, conversation_id: &AIConversationId) -> Option<&ConversationStatus> {
        self.conversation(conversation_id).map(|c| c.status())
    }

    pub fn all_live_conversations_for_terminal_view(&self, _terminal_view_id: EntityId) -> impl Iterator<Item = &AIConversation> {
        std::iter::empty()
    }

    pub fn all_live_root_task_exchanges_for_terminal_view(&self, _terminal_view_id: EntityId) -> impl Iterator<Item = &AIAgentExchange> {
        std::iter::empty()
    }

    pub fn all_cleared_root_task_exchanges_for_terminal_view(&self, _terminal_view_id: EntityId) -> impl Iterator<Item = &AIAgentExchange> {
        std::iter::empty()
    }

    pub fn all_cleared_conversations(&self) -> Vec<(EntityId, &AIConversation)> {
        vec![]
    }

    pub fn all_live_conversations(&self) -> Vec<(EntityId, &AIConversation)> {
        vec![]
    }

    pub fn child_conversations_of(&self, _parent_id: AIConversationId) -> Vec<&AIConversation> {
        vec![]
    }

    pub fn child_conversation_ids_of(&self, _parent_id: &AIConversationId) -> &[AIConversationId] {
        &self.empty_conversation_ids
    }

    pub fn resolved_parent_conversation_id_for_conversation(&self, _conversation: &AIConversation) -> Option<AIConversationId> {
        None
    }

    pub fn is_entirely_passive_conversation(&self, conversation_id: &AIConversationId) -> bool {
        self.conversation(conversation_id).is_some_and(|c| c.is_entirely_passive())
    }

    pub fn is_exchange_hidden(&self, _conversation_id: AIConversationId, _exchange_id: AIAgentExchangeId) -> bool {
        false
    }

    pub fn is_conversation_live(&self, _conversation_id: AIConversationId) -> bool {
        false
    }

    pub fn existing_suggestions_for_conversation(&self, conversation_id: AIConversationId) -> Option<&Suggestions> {
        self.conversations_by_id.get(&conversation_id).and_then(|c| c.existing_suggestions())
    }

    pub fn conversation_for_response_stream(&self, _response_stream_id: &ResponseStreamId) -> Option<AIConversationId> {
        None
    }

    pub fn conversation_id_for_action(&self, _action_id: &AIAgentActionId, _terminal_view_id: EntityId) -> Option<AIConversationId> {
        None
    }

    pub fn conversation_id_for_exchange(&self, _exchange_id: AIAgentExchangeId) -> Option<AIConversationId> {
        None
    }

    pub fn conversation_id_for_agent_id(&self, _agent_id: &str) -> Option<AIConversationId> {
        None
    }

    pub fn latest_exchange_across_all_conversations(&self, _terminal_view_id: EntityId) -> Option<&AIAgentExchange> {
        None
    }

    pub fn can_conversation_be_shared(&self, _conversation_id: &AIConversationId) -> bool {
        false
    }

    pub fn merge_cloud_conversation_metadata(&mut self, _metadata: Vec<ServerAIConversationMetadata>) {}

    pub fn get_local_conversations_metadata(&self) -> impl Iterator<Item = &AIConversationMetadata> {
        std::iter::empty()
    }

    pub fn get_conversation_metadata(&self, _conversation_id: &AIConversationId) -> Option<&AIConversationMetadata> {
        None
    }

    pub fn get_server_conversation_metadata(&self, _conversation_id: &AIConversationId) -> Option<&ServerAIConversationMetadata> {
        None
    }

    pub fn get_server_conversation_metadata_by_server_token(&self, _token: &ServerConversationToken) -> Option<&ServerAIConversationMetadata> {
        None
    }

    pub fn find_conversation_id_by_server_token(&self, _token: &ServerConversationToken) -> Option<AIConversationId> {
        None
    }

    pub fn get_or_set_canonical_conversation_id_for_server_token(&mut self, _token: &ServerConversationToken) -> AIConversationId {
        AIConversationId::new()
    }

    pub(crate) fn all_ai_queries(&self, _terminal_view_id: Option<EntityId>) -> impl Iterator<Item = super::AIQueryHistory> {
        std::iter::empty()
    }

    pub fn load_conversation_by_server_token<C>(
        &mut self,
        _token: &ServerConversationToken,
        _ctx: &mut C,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<CloudConversationData>> + Send>> {
        Box::pin(std::future::ready(None))
    }

    pub fn load_conversation_data(
        &self,
        _conversation_id: AIConversationId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<CloudConversationData>> + Send>> {
        Box::pin(std::future::ready(None))
    }

    pub fn restore_conversations(&mut self, _terminal_view_id: EntityId, _conversations: Vec<AIConversation>, _ctx: &mut ModelContext<Self>) {}

    pub fn start_new_conversation(&mut self, _terminal_view_id: EntityId, _is_autoexecute_override: bool, _is_viewing_shared_session: bool, _is_cli_agent_transcript: bool, _ctx: &mut ModelContext<Self>) -> AIConversationId {
        AIConversationId::new()
    }

    pub fn set_active_conversation_id(&mut self, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn update_conversation_status(&mut self, _terminal_view_id: EntityId, _conversation_id: AIConversationId, _status: ConversationStatus, _ctx: &mut ModelContext<Self>) {}

    pub fn update_conversation_status_with_error_message(&mut self, _terminal_view_id: EntityId, _conversation_id: AIConversationId, _status: ConversationStatus, _error_message: Option<String>, _ctx: &mut ModelContext<Self>) {}

    pub(crate) fn clear_conversations_in_terminal_view(&mut self, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn remove_conversation(&mut self, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn delete_conversation(&mut self, _conversation_id: AIConversationId, _terminal_view_id: Option<EntityId>, _ctx: &mut ModelContext<Self>) {}

    pub fn fork_conversation(&mut self, _conversation: &AIConversation, _prefix: &str, _preserve_task_ids: bool, _title_override: Option<&str>, _ctx: &mut ModelContext<Self>) -> Result<AIConversation, String> { Err("stubs: fork_conversation not available".into()) }

    pub fn fork_conversation_at_exchange(&mut self, _conversation: &AIConversation, _exchange_id: AIAgentExchangeId, _fork_from_exact: bool, _prefix: &str, _title_override: Option<&str>, _ctx: &mut ModelContext<Self>) -> Result<AIConversation, String> { Err("stubs: fork_conversation_at_exchange not available".into()) }

    pub fn on_forked_conversation(&mut self, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn initialize_output_for_response_stream(&mut self, _stream_id: ResponseStreamId, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn assign_run_id_for_conversation(&mut self, _conversation_id: AIConversationId, _run_id: String, _task_id: Option<crate::ai::ambient_agents::AmbientAgentTaskId>, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn mark_response_stream_completed_successfully(&mut self, _stream_id: &ResponseStreamId, _ctx: &mut ModelContext<Self>) {}

    pub fn mark_response_stream_cancelled(&mut self, _stream_id: &ResponseStreamId, _reason: crate::ai::agent::CancellationReason, _ctx: &mut ModelContext<Self>) {}

    pub fn mark_response_stream_completed_with_error(&mut self, _stream_id: &ResponseStreamId, _error: crate::ai::agent::RenderableAIError, _ctx: &mut ModelContext<Self>) {}

    pub fn set_exchange_time_to_first_token(&mut self, _exchange_id: AIAgentExchangeId, _ctx: &mut ModelContext<Self>) {}

    pub fn update_conversation_cost_and_usage_for_request(&mut self, _stream_id: &ResponseStreamId, _request_cost: crate::ai::agent::RequestCost, _ctx: &mut ModelContext<Self>) {}

    pub fn apply_client_actions(&mut self, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _actions: Vec<warp_multi_agent_api::client_action::Action>, _ctx: &mut ModelContext<Self>) -> Vec<warp_multi_agent_api::client_action::Action> { vec![] }

    pub fn mark_conversation_as_remote_child(&mut self, _conversation_id: AIConversationId, _ctx: &mut ModelContext<Self>) {}

    pub fn update_event_sequence(&mut self, _conversation_id: AIConversationId, _ctx: &mut ModelContext<Self>) {}

    pub fn set_conversation_pinned(&mut self, _conversation_id: AIConversationId, _is_pinned: bool, _ctx: &mut ModelContext<Self>) {}

    pub fn set_server_conversation_token_for_conversation(&mut self, _conversation_id: AIConversationId, _token: impl Into<String>) {}

    pub fn set_server_conversation_token_for_conversation_and_persist(&mut self, _conversation_id: AIConversationId, _token: impl Into<String>, _ctx: &mut ModelContext<Self>) {}

    pub fn set_server_metadata_for_conversation(&mut self, _conversation_id: AIConversationId, _metadata: ServerAIConversationMetadata, _ctx: &mut ModelContext<Self>) {}

    pub fn set_parent_for_conversation(&mut self, _conversation_id: AIConversationId, _parent_id: AIConversationId) {}

    pub fn start_new_child_conversation(&mut self, _terminal_view_id: EntityId, _name: String, _parent_conversation_id: AIConversationId, _orchestration_harness: Option<warp_cli::agent::Harness>, _ctx: &mut ModelContext<Self>) -> AIConversationId { AIConversationId::new() }

    pub(super) fn update_conversation_for_new_request_input(&mut self, _request_input: super::RequestInput, _stream_id: ResponseStreamId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) -> Result<(), UpdateHistoryError> { Ok(()) }

    pub fn create_cli_subagent_task_for_conversation(&mut self, _block_id: BlockId, _conversation_id: AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) -> Result<TaskId, UpdateHistoryError> { Err(UpdateHistoryError::ConversationNotFound(AIConversationId::new())) }

    pub fn mark_terminal_view_as_ambient_agent_session_view(&mut self, _terminal_view_id: EntityId) {}

    pub fn mark_terminal_view_as_conversation_transcript_viewer(&mut self, _terminal_view_id: EntityId) {}

    pub fn is_terminal_view_conversation_transcript_viewer(&self, _terminal_view_id: EntityId) -> bool { false }

    pub fn mark_conversations_historical_for_terminal_view(&mut self, _terminal_view_id: EntityId) {}

    pub fn set_exchange_hidden_status(&mut self, _conversation_id: AIConversationId, _exchange_id: AIAgentExchangeId, _is_hidden: bool, _ctx: &mut ModelContext<Self>) {}

    pub fn set_viewing_shared_session_for_conversation(&mut self, _conversation_id: AIConversationId, _is_viewing: bool) {}

    pub fn set_has_code_review_opened_to_true(&mut self, _conversation_id: AIConversationId) {}

    pub fn toggle_autoexecute_override(&mut self, _conversation_id: &AIConversationId, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) {}

    pub fn truncate_conversation_from_exchange(&mut self, _conversation_id: AIConversationId, _exchange_id: AIAgentExchangeId, _ctx: &mut ModelContext<Self>) -> Result<std::collections::HashSet<AIAgentExchangeId>, String> { Ok(std::collections::HashSet::new()) }

    pub fn insert_forked_conversation_from_tasks(&mut self, _conversation: AIConversation, _terminal_view_id: EntityId, _ctx: &mut ModelContext<Self>) -> AIConversationId { AIConversationId::new() }

    pub(crate) fn reset(&mut self) { *self = Self::default(); }

    pub fn record_new_conversation_request_complete(&mut self, request_id: StartAgentRequestId, conversation_id: AIConversationId, ctx: &mut ModelContext<Self>) {
        ctx.emit(BlocklistAIHistoryEvent::NewConversationRequestComplete { request_id, conversation_id });
    }
}
