use parking_lot::FairMutex;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use warpui::{Entity, EntityId, ModelContext, ModelHandle};

use crate::ai::agent::conversation_types::AIConversationId;
use crate::ai::agent::task::TaskId;
use crate::ai::agent::AIAgentActionId;
use crate::terminal::model::block::BlockId;
use crate::terminal::model_events::ModelEventDispatcher;
use crate::terminal::TerminalModel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum UserTakeOverReason {
    Manual,
    Stop,
    TransferFromAgent { reason: String },
}

impl UserTakeOverReason {
    pub fn is_stop(&self) -> bool {
        matches!(self, Self::Stop)
    }

    pub fn is_transfer_from_agent(&self) -> bool {
        matches!(self, Self::TransferFromAgent { .. })
    }

    pub fn transfer_reason(&self) -> Option<&str> {
        match self {
            Self::TransferFromAgent { reason } => Some(reason.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LongRunningCommandControlState {
    Agent { is_blocked: bool, should_hide_responses: bool },
    User { reason: UserTakeOverReason },
}

impl LongRunningCommandControlState {
    pub fn is_agent_in_control(&self) -> bool {
        matches!(self, Self::Agent { .. })
    }

    pub fn is_agent_blocked(&self) -> bool {
        matches!(self, Self::Agent { is_blocked: true, .. })
    }

    pub fn is_user_in_control(&self) -> bool {
        matches!(self, Self::User { .. })
    }

    pub fn should_hide_responses(&self) -> bool {
        matches!(self, Self::Agent { should_hide_responses: true, .. })
    }

    pub fn user_take_over_reason(&self) -> Option<&UserTakeOverReason> {
        match self {
            LongRunningCommandControlState::Agent { .. } => None,
            LongRunningCommandControlState::User { reason } => Some(reason),
        }
    }
}

pub enum CLISubagentEvent {
    SpawnedSubagent {
        task_id: TaskId,
        block_id: BlockId,
        conversation_id: AIConversationId,
        initial_requested_command_action_id: Option<AIAgentActionId>,
    },
    FinishedSubagent {
        block_id: BlockId,
        conversation_id: Option<AIConversationId>,
        initial_requested_command_action_id: Option<AIAgentActionId>,
    },
    UpdatedControl {
        block_id: BlockId,
        requested_command_action_id: Option<AIAgentActionId>,
        agent_has_control: bool,
    },
    UpdatedLastSnapshot,
    ToggledHideResponses,
    ControlHandedBackAfterTransfer,
}

pub struct CLISubagentController;

impl CLISubagentController {
    pub fn new(
        _terminal_model: Arc<FairMutex<TerminalModel>>,
        _model_event_dispatcher: &ModelHandle<ModelEventDispatcher>,
        _terminal_view_id: EntityId,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        CLISubagentController
    }

    pub fn is_agent_in_control(&self) -> bool {
        false
    }

    pub fn last_snapshot_at(
        &self,
        _block_id: &BlockId,
    ) -> Option<instant::Instant> {
        None
    }

    pub fn switch_control_to_user(
        &self,
        _reason: UserTakeOverReason,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn handoff_active_command_control_to_agent(
        &self,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn toggle_hide_responses(&self, _ctx: &mut ModelContext<Self>) {}

    pub fn is_agent_in_control_or_tagged_in(&self) -> bool { false }
}

impl Entity for CLISubagentController {
    type Event = CLISubagentEvent;
}
