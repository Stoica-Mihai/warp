use std::collections::HashMap;

use chrono::{DateTime, Local};
use warp_multi_agent_api::ToolType;

use crate::ai::agent::AIAgentInput;
use crate::ai::conversation_types::AIConversationId;
use crate::ai::agent_types::TaskId;
use ai::LLMId;

#[derive(Debug)]
pub struct RequestInput {
    pub conversation_id: AIConversationId,
    pub input_messages: HashMap<TaskId, Vec<AIAgentInput>>,
    pub working_directory: Option<String>,
    pub model_id: LLMId,
    pub coding_model_id: LLMId,
    pub cli_agent_model_id: LLMId,
    pub computer_use_model_id: LLMId,
    pub request_start_ts: DateTime<Local>,
    pub supported_tools_override: Option<Vec<ToolType>>,
}

impl RequestInput {
    pub fn all_inputs(&self) -> impl Iterator<Item = &AIAgentInput> {
        self.input_messages.values().flatten()
    }

    pub fn with_supported_tools(mut self, tools: Vec<ToolType>) -> Self {
        self.supported_tools_override = Some(tools);
        self
    }
}
