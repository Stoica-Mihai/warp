use std::cmp::Ordering;
use std::collections::HashSet;

use chrono::TimeZone;
use warpui::{AppContext, EntityId, SingletonEntity, WindowId};

use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::{AIConversation, AIConversationId};
use crate::terminal::view::blocklist_filter;
use crate::workspace::{PaneViewLocator};

/// Result from matching a conversation.
/// terminal_view_id and window_id are optional because, when we add restored conversations,
/// these conversations will not have associated windows or terminal views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationNavigationData {
    pub id: AIConversationId,
    pub title: String,
    pub initial_query: Option<String>,
    pub last_updated: chrono::DateTime<chrono::Local>,
    pub terminal_view_id: Option<EntityId>,
    pub window_id: Option<WindowId>,
    pub pane_view_locator: Option<PaneViewLocator>,
    pub initial_working_directory: Option<String>,
    pub latest_working_directory: Option<String>,
    pub is_selected: bool,
    pub is_in_active_pane: bool,
    /// The conversation is hidden on the undo stack if its parent view (either the tab or the split pane)
    /// has been recently closed, and this closure can still be undone. We should still show the conversation
    /// as historical even though the pane group still "exists", as the pane group is still hidden to the user.
    pub is_closed: bool,
    /// The server-generated conversation token, used to reference this conversation in context tags.
    pub server_conversation_token: Option<ServerConversationToken>,
}

impl PartialOrd for ConversationNavigationData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConversationNavigationData {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_historical(), other.is_historical()) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            _ => match (self.is_in_active_pane, other.is_in_active_pane) {
                (false, true) => Ordering::Less,
                (true, false) => Ordering::Greater,
                _ => match (self.is_selected, other.is_selected) {
                    (false, true) => Ordering::Less,
                    (true, false) => Ordering::Greater,
                    _ => self.last_updated.cmp(&other.last_updated),
                },
            },
        }
    }
}

impl ConversationNavigationData {
    #[allow(clippy::too_many_arguments)]
    pub fn from_ai_conversation(
        conversation: &AIConversation,
        terminal_view_id: Option<EntityId>,
        window_id: Option<WindowId>,
        pane_view_locator: Option<PaneViewLocator>,
        initial_working_directory: Option<String>,
        is_selected: bool,
        is_in_active_pane: bool,
        is_closed: bool,
    ) -> Self {
        let initial_query = conversation.initial_query();
        let title = conversation
            .title()
            .unwrap_or_else(|| "Untitled conversation".to_string());
        let last_updated = conversation
            .latest_exchange()
            .map(|exchange| exchange.start_time)
            .unwrap_or_else(chrono::Local::now);

        Self {
            id: conversation.id(),
            title,
            initial_query,
            last_updated,
            terminal_view_id,
            window_id,
            pane_view_locator,
            initial_working_directory,
            latest_working_directory: conversation.current_working_directory(),
            is_selected,
            is_in_active_pane,
            is_closed,
            server_conversation_token: conversation.server_conversation_token().cloned(),
        }
    }


    pub fn id(&self) -> AIConversationId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn last_updated(&self) -> chrono::DateTime<chrono::Local> {
        self.last_updated
    }

    pub fn pane_view_locator(&self) -> Option<PaneViewLocator> {
        self.pane_view_locator
    }

    pub fn is_in_active_pane(&self) -> bool {
        self.is_in_active_pane
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window_id
    }

    // A conversation is historical if it does not have an open terminal view associated with it
    pub fn is_historical(&self) -> bool {
        self.terminal_view_id.is_none() || self.is_closed
    }

    pub fn all_conversations(app: &AppContext) -> Vec<ConversationNavigationData> {
        let _ = app;
        Vec::new()
    }

    pub fn historical_conversations(app: &AppContext) -> Vec<ConversationNavigationData> {
        let _ = app;
        Vec::new()
    }
}
