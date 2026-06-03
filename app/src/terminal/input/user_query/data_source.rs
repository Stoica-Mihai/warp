//! Data source for the user query menu.

use warpui::{AppContext, Entity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::AIAgentExchangeId;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::SyncDataSource;

/// Action emitted when a query is selected in the user query menu.
#[derive(Clone, Debug)]
pub struct SelectUserQuery {
    pub exchange_id: AIAgentExchangeId,
}

pub struct UserQueryDataSource {
    conversation_id: AIConversationId,
}

impl UserQueryDataSource {
    pub fn new(conversation_id: AIConversationId) -> Self {
        Self { conversation_id }
    }

    pub fn set_conversation_id(&mut self, conversation_id: AIConversationId) {
        self.conversation_id = conversation_id;
    }
}

impl SyncDataSource for UserQueryDataSource {
    type Action = SelectUserQuery;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let _ = (query, app);
        Ok(vec![])
    }
}

impl Entity for UserQueryDataSource {
    type Event = ();
}
