//! Data source for the inline history menu, providing both conversations and commands.
//!
//! Ordering semantics match the legacy up-arrow history menu:
//! - Items from different sessions appear before items from the current session
//! - Within each group, items are sorted by timestamp (oldest first)
//! - Commands are deduplicated, keeping the most recent occurrence
//! - The result is that current session items appear at the bottom (closer to input)

use chrono::Local;
use ordered_float::OrderedFloat;
use warpui::{AppContext, Entity, EntityId, ModelHandle, SingletonEntity};

use crate::input_suggestions::HistoryInputSuggestion;
use crate::search::data_source::{Query, QueryFilter, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::SyncDataSource;
use crate::terminal::history::{History, LinkedWorkflowData, UpArrowHistoryConfig};
use crate::terminal::input::inline_history::search_item::InlineHistoryItem;
use crate::terminal::input::inline_menu::{
    InlineMenuAction, InlineMenuClickBehavior, InlineMenuType,
};
use crate::terminal::model::session::active_session::ActiveSession;
use crate::ai::agent::conversation::AIConversationId;

#[derive(Clone, Debug)]
pub enum AcceptHistoryItem {
    Conversation {
        conversation_id: AIConversationId,
        title: String,
    },
    Command {
        command: String,
        linked_workflow_data: Option<LinkedWorkflowData>,
    },
}

impl AcceptHistoryItem {
    pub fn buffer_replacement_text(&self) -> Option<&String> {
        match self {
            AcceptHistoryItem::Command { command, .. } => Some(command),
            AcceptHistoryItem::Conversation { .. } => None,
        }
    }
}

impl InlineMenuAction for AcceptHistoryItem {
    const MENU_TYPE: InlineMenuType = InlineMenuType::InlineHistoryMenu;

    fn click_behavior(&self) -> InlineMenuClickBehavior {
        match self {
            AcceptHistoryItem::Conversation { .. } => InlineMenuClickBehavior::AcceptOnClick,
            AcceptHistoryItem::Command { .. } => InlineMenuClickBehavior::SelectOnClick,
        }
    }
}

/// Data source that provides both live conversations for a terminal view and command history.
pub struct InlineHistoryMenuDataSource {
    terminal_view_id: EntityId,
    active_session: ModelHandle<ActiveSession>,
}

impl InlineHistoryMenuDataSource {
    pub fn new(terminal_view_id: EntityId, active_session: ModelHandle<ActiveSession>) -> Self {
        Self {
            terminal_view_id,
            active_session,
        }
    }

}


impl SyncDataSource for InlineHistoryMenuDataSource {
    type Action = AcceptHistoryItem;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let trimmed_query = query.text.trim();
        let prefix_match_len = trimmed_query.len();

        let session_id = self.active_session.as_ref(app).session(app).map(|s| s.id());

        let include_commands =
            query.filters.is_empty() || query.filters.contains(&QueryFilter::Commands);

        let history = History::handle(app).as_ref(app);

        let mut results: Vec<QueryResult<AcceptHistoryItem>> = Vec::new();
        if include_commands {
            for suggestion in history.up_arrow_suggestions_for_terminal_view(
                self.terminal_view_id,
                session_id,
                UpArrowHistoryConfig {
                    include_commands: true,
                    include_prompts: false,
                },
                app,
            ) {
                let HistoryInputSuggestion::Command { entry } = &suggestion;
                let command = entry.command.trim();
                if command.is_empty() {
                    continue;
                }
                if !trimmed_query.is_empty() && !command.starts_with(trimmed_query) {
                    continue;
                }
                let display_timestamp = entry.start_ts.unwrap_or_else(Local::now);
                let search_item = InlineHistoryItem::command(
                    command.to_string(),
                    entry.linked_workflow_data(),
                    display_timestamp,
                )
                .with_prefix_match_len(prefix_match_len);
                let score = OrderedFloat(results.len() as f64);
                results.push(QueryResult::from(search_item.with_score(score)));
            }
        }

        Ok(results)
    }
}

impl Entity for InlineHistoryMenuDataSource {
    type Event = ();
}

#[cfg(test)]
#[path = "data_source_tests.rs"]
mod tests;
