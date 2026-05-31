use std::sync::Arc;

use ai::agent::action::{RunAgentsAgentRunConfig, RunAgentsExecutionMode, RunAgentsRequest};
use ai::skills::SkillReference;
use warp_cli::agent::Harness;
use warpui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::{
    AIAgentAction, AIAgentActionId, AIAgentActionResult, CancellationReason,
    StartAgentExecutionMode, SuggestPromptResult,
};
use crate::ai::local_child_harnesses::local_child_harness_disabled_message;

#[derive(Debug, Clone)]
pub enum AIActionStatus {
    Preprocessing,
    Queued,
    Blocked,
    RunningAsync,
    Finished(Arc<AIAgentActionResult>),
}

impl AIActionStatus {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::RunningAsync | Self::Preprocessing | Self::Queued)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Finished(_))
    }

    pub fn is_queued(&self) -> bool { matches!(self, Self::Queued) }
    pub fn is_preprocessing(&self) -> bool { matches!(self, Self::Preprocessing) }
    pub fn is_cancelled(&self) -> bool { false }
    pub fn is_failed(&self) -> bool { false }
    pub fn is_success(&self) -> bool { false }
    pub fn finished_result(&self) -> Option<&Arc<AIAgentActionResult>> { None }
    pub fn is_cancelled_during_requested_command_execution(&self) -> bool { false }
}

pub struct ShellCommandExecutor;

impl ShellCommandExecutor {
    pub const MAX_AGENT_DELAY_DURATION: std::time::Duration = std::time::Duration::from_secs(60);

    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn force_refresh_block(&mut self, _block_id: &crate::terminal::model::BlockId) {}
}

impl Entity for ShellCommandExecutor {
    type Event = ShellCommandExecutorEvent;
}

impl SingletonEntity for ShellCommandExecutor {}

pub enum ShellCommandExecutorEvent {}

pub struct RunAgentsExecutor;

impl RunAgentsExecutor {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }
}

impl Entity for RunAgentsExecutor {
    type Event = RunAgentsExecutorEvent;
}

impl SingletonEntity for RunAgentsExecutor {}

#[derive(Debug, Clone, Copy)]
pub struct RunAgentsSpawningSnapshot {
    pub agent_count: usize,
}

pub enum RunAgentsExecutorEvent {
    SpawningStarted {
        action_id: AIAgentActionId,
        snapshot: Box<RunAgentsSpawningSnapshot>,
    },
    SpawningFinished {
        action_id: AIAgentActionId,
    },
}

pub struct SuggestPromptExecutor;

impl SuggestPromptExecutor {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn complete_suggest_prompt_action(&mut self, _result: SuggestPromptResult) {}
}

impl Entity for SuggestPromptExecutor {
    type Event = ();
}

impl SingletonEntity for SuggestPromptExecutor {}

pub struct SuggestNewConversationExecutor;

impl SuggestNewConversationExecutor {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn complete_suggest_new_conversation_action(&mut self, _decision: NewConversationDecision) {}
}

impl Entity for SuggestNewConversationExecutor {
    type Event = ();
}

impl SingletonEntity for SuggestNewConversationExecutor {}

pub struct AskUserQuestionExecutor;
impl AskUserQuestionExecutor { pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self } }
impl Entity for AskUserQuestionExecutor { type Event = (); }
impl SingletonEntity for AskUserQuestionExecutor {}

pub struct RequestFileEditsExecutor;
impl RequestFileEditsExecutor { pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self } }
impl Entity for RequestFileEditsExecutor { type Event = (); }
impl SingletonEntity for RequestFileEditsExecutor {}

pub struct SearchCodebaseExecutor;
impl SearchCodebaseExecutor { pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self } }
impl Entity for SearchCodebaseExecutor { type Event = (); }
impl SingletonEntity for SearchCodebaseExecutor {}

pub struct BlocklistAIActionModel;

impl BlocklistAIActionModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn is_view_only(&self) -> bool {
        false
    }

    pub fn get_action_status(&self, _action_id: &AIAgentActionId) -> Option<AIActionStatus> {
        None
    }

    pub fn get_action_result(
        &self,
        _action_id: &AIAgentActionId,
    ) -> Option<Arc<AIAgentActionResult>> {
        None
    }

    pub fn get_pending_actions(&self) -> &[AIAgentAction] {
        &[]
    }

    pub fn get_pending_or_running_action_id(&self, _ctx: &AppContext) -> Option<&AIAgentActionId> {
        None
    }

    pub fn cancel_action_with_id(
        &mut self,
        _conversation_id: AIConversationId,
        _action_id: &AIAgentActionId,
        _reason: CancellationReason,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn shell_command_executor(&self, ctx: &AppContext) -> ModelHandle<ShellCommandExecutor> {
        ShellCommandExecutor::handle(ctx)
    }

    pub fn run_agents_executor(&self, ctx: &AppContext) -> ModelHandle<RunAgentsExecutor> {
        RunAgentsExecutor::handle(ctx)
    }

    pub fn suggest_prompt_executor(
        &self,
        ctx: &AppContext,
    ) -> ModelHandle<SuggestPromptExecutor> {
        SuggestPromptExecutor::handle(ctx)
    }

    pub fn suggest_new_conversation_executor(
        &self,
        ctx: &AppContext,
    ) -> ModelHandle<SuggestNewConversationExecutor> {
        SuggestNewConversationExecutor::handle(ctx)
    }

    pub fn execute_action(
        &mut self,
        _action_id: &AIAgentActionId,
        _conversation_id: AIConversationId,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn handle_requested_command_accepted(
        &mut self,
        _action_id: &AIAgentActionId,
        _command_text: String,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn execute_next_action_for_user(
        &mut self,
        _conversation_id: AIConversationId,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn execute_run_agents(
        &mut self,
        _action_id: &AIAgentActionId,
        _request: RunAgentsRequest,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn ask_user_question_executor(&mut self, _ctx: &AppContext) -> ModelHandle<AskUserQuestionExecutor> {
        AskUserQuestionExecutor::handle(_ctx)
    }

    pub fn deny_run_agents(&mut self, _action_id: &AIAgentActionId, _ctx: &mut ModelContext<Self>) {}

    pub fn get_async_running_action(&self, _conversation_id: &crate::ai::agent::conversation::AIConversationId) -> Option<&AIAgentActionId> { None }

    pub fn get_finished_action_results(&self, _conversation_id: &crate::ai::agent::conversation::AIConversationId) -> impl Iterator<Item = (&AIAgentActionId, &AIAgentActionResult)> {
        std::iter::empty()
    }

    pub fn get_pending_action(&self, _action_id: &AIAgentActionId) -> Option<&ai::agent::action::AIAgentAction> { None }

    pub fn get_pending_actions_for_conversation(&self, _id: &crate::ai::agent::conversation::AIConversationId) -> impl Iterator<Item = &ai::agent::action::AIAgentAction> {
        std::iter::empty()
    }

    pub fn has_unfinished_actions_for_conversation(&self, _id: &crate::ai::agent::conversation::AIConversationId) -> bool { false }

    pub fn request_file_edits_executor(&self, _ctx: &AppContext) -> ModelHandle<RequestFileEditsExecutor> {
        RequestFileEditsExecutor::handle(_ctx)
    }

    pub fn search_codebase_executor(&self, _ctx: &AppContext) -> ModelHandle<SearchCodebaseExecutor> {
        SearchCodebaseExecutor::handle(_ctx)
    }
}

impl Entity for BlocklistAIActionModel {
    type Event = BlocklistAIActionEvent;
}

impl SingletonEntity for BlocklistAIActionModel {}

pub enum BlocklistAIActionEvent {
    ActionBlockedOnUserConfirmation(AIAgentActionId),
    ExecutingAction(AIAgentActionId),
    FinishedAction { action_id: AIAgentActionId, success: bool },
    CancelledAction,
    QueuedAction(AIAgentActionId),
    InsertCodeReviewComments(AIAgentActionId),
    InitProject(AIAgentActionId),
    ToggleCodeReview(AIAgentActionId),
}

impl BlocklistAIActionEvent {
    pub fn action_id(&self) -> &AIAgentActionId {
        match self {
            Self::ActionBlockedOnUserConfirmation(id)
            | Self::ExecutingAction(id)
            | Self::QueuedAction(id)
            | Self::InsertCodeReviewComments(id)
            | Self::InitProject(id)
            | Self::ToggleCodeReview(id) => id,
            Self::FinishedAction { action_id, .. } => action_id,
            Self::CancelledAction => panic!("CancelledAction has no action_id"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewConversationDecision {
    Accept,
    Reject,
}

pub fn compose_run_agents_child_prompt(base_prompt: &str, per_agent_prompt: &str) -> String {
    let base = base_prompt.trim();
    let per = per_agent_prompt.trim();
    match (base.is_empty(), per.is_empty()) {
        (true, true) => String::new(),
        (true, false) => per.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}\n\n{per}"),
    }
}

pub fn run_agents_to_start_agent_mode(
    execution_mode: &RunAgentsExecutionMode,
    harness_type: &str,
    model_id: &str,
    skills: &[SkillReference],
    auth_secret_name: Option<&str>,
    agent_cfg: &RunAgentsAgentRunConfig,
) -> Result<StartAgentExecutionMode, String> {
    match execution_mode {
        RunAgentsExecutionMode::Remote {
            environment_id,
            worker_host,
            computer_use_enabled,
        } => {
            if harness_type == "opencode" {
                return Err(format!(
                    "opencode is not supported as a remote child harness"
                ));
            }
            let secret = auth_secret_name
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            Ok(StartAgentExecutionMode::Remote {
                environment_id: environment_id.clone(),
                worker_host: worker_host.clone(),
                computer_use_enabled: *computer_use_enabled,
                harness_type: harness_type.to_string(),
                model_id: model_id.to_string(),
                title: agent_cfg.title.clone(),
                skill_references: skills.to_vec(),
                auth_secret_name: secret,
            })
        }
        RunAgentsExecutionMode::Local => {
            let harness = Harness::parse_local_child_harness(harness_type);
            if let Some(harness) = harness {
                if let Some(msg) = local_child_harness_disabled_message(harness) {
                    return Err(msg.to_string());
                }
                Ok(StartAgentExecutionMode::Local {
                    harness_type: Some(harness_type.to_string()),
                    model_id: Some(model_id.to_string()),
                })
            } else {
                Ok(StartAgentExecutionMode::local_with_defaults())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StartAgentRequestId(u64);

#[derive(Debug, Clone)]
pub struct StartAgentRequest {
    pub id: StartAgentRequestId,
    pub name: String,
    pub prompt: String,
    pub execution_mode: StartAgentExecutionMode,
    pub lifecycle_subscription: Option<Vec<crate::ai::agent::LifecycleEventType>>,
    pub parent_conversation_id: AIConversationId,
    pub parent_run_id: Option<String>,
}

pub struct StartAgentExecutor;
impl StartAgentExecutor {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }
}
impl Entity for StartAgentExecutor { type Event = StartAgentExecutorEvent; }
impl SingletonEntity for StartAgentExecutor {}
pub enum StartAgentExecutorEvent {}

#[derive(Debug)]
pub struct ReadFileContextResult {
    pub file_contexts: Vec<ai::agent::action_result::FileContext>,
    pub missing_files: Vec<String>,
}

pub async fn read_local_file_context(
    _file_locations: &[crate::ai::agent::FileLocations],
    _a: Option<usize>,
    _b: Option<usize>,
    _max_file_bytes: Option<usize>,
    _max_batch_bytes: Option<usize>,
) -> anyhow::Result<ReadFileContextResult> {
    Err(anyhow::anyhow!("AI file context reading not available"))
}

pub fn coerce_integer_args(
    map: &mut serde_json::Map<String, serde_json::Value>,
    schema: &serde_json::Map<String, serde_json::Value>,
) {
    let Some(serde_json::Value::Object(properties)) = schema.get("properties") else {
        return;
    };
    for (key, prop_schema) in properties {
        let serde_json::Value::Object(prop_obj) = prop_schema else {
            continue;
        };
        let is_integer = prop_obj
            .get("type")
            .and_then(|t| t.as_str())
            .is_some_and(|t| t == "integer");
        if !is_integer {
            continue;
        }
        if let Some(val) = map.get_mut(key) {
            if let Some(f) = val.as_f64() {
                *val = serde_json::Value::Number(
                    serde_json::Number::from(f as i64),
                );
            }
        }
    }
}
