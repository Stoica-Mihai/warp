//! Data source for the rewind menu.

use std::collections::HashSet;

use ai::agent::action_result::RequestFileEditsResult;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use warpui::{AppContext, Entity, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::{AIAgentActionResultType, AIAgentExchangeId};
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::SyncDataSource;

/// Action emitted when a rewind point is selected.
#[derive(Clone, Debug)]
pub struct SelectRewindPoint {
    /// The exchange ID to rewind to, or None for "Current" (dismiss without rewinding).
    pub exchange_id: Option<AIAgentExchangeId>,
}

/// Information about file changes for a rewind point.
#[derive(Debug, Clone, Default)]
pub struct FileChangesInfo {
    pub lines_added: usize,
    pub lines_removed: usize,
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

    /// Get file changes info for a "block" of exchanges (from user query to next user query)
    fn get_file_changes_for_block(
        exchanges: &[&crate::ai::agent::AIAgentExchange],
    ) -> FileChangesInfo {
        let mut file_paths: HashSet<String> = HashSet::new();
        let mut total_lines_added: usize = 0;
        let mut total_lines_removed: usize = 0;

        for exchange in exchanges {
            for input in &exchange.input {
                if let Some(action_result) = input.action_result() {
                    if let AIAgentActionResultType::RequestFileEdits(
                        RequestFileEditsResult::Success {
                            updated_files,
                            lines_added,
                            lines_removed,
                            ..
                        },
                    ) = &action_result.result
                    {
                        total_lines_added += lines_added;
                        total_lines_removed += lines_removed;

                        for updated_file in updated_files {
                            file_paths.insert(updated_file.file_context.file_name.clone());
                        }
                    }
                }
            }
        }

        if file_paths.is_empty() {
            return FileChangesInfo::default();
        }

        FileChangesInfo {
            lines_added: total_lines_added,
            lines_removed: total_lines_removed,
        }
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
