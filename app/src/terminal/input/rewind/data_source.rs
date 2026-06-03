//! Data source for the rewind menu.

use warpui::{AppContext, Entity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::AIAgentExchangeId;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::SyncDataSource;

/// Action emitted when a rewind point is selected.
#[derive(Clone, Debug)]
pub struct SelectRewindPoint {
    /// The exchange ID to rewind to, or None for "Current" (dismiss without rewinding).
    pub exchange_id: Option<AIAgentExchangeId>,
}

pub struct RewindDataSource {
    conversation_id: AIConversationId,
}

impl RewindDataSource {
    pub fn new(conversation_id: AIConversationId) -> Self {
        Self { conversation_id }
    }

    pub fn set_conversation_id(&mut self, conversation_id: AIConversationId) {
        self.conversation_id = conversation_id;
    }
}

impl SyncDataSource for RewindDataSource {
    type Action = SelectRewindPoint;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let _ = (query, app);
        Ok(vec![])
    }
}

impl Entity for RewindDataSource {
    type Event = ();
}
