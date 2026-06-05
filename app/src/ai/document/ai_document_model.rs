pub use ai::document::{AIDocumentId, AIDocumentVersion};

use chrono::{DateTime, Local};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;

pub enum AIDocumentModelEvent {}

pub struct AIDocument {
    pub title: String,
    pub version: AIDocumentVersion,
    pub created_at: DateTime<Local>,
}

pub struct AIDocumentModel;

impl Entity for AIDocumentModel {
    type Event = AIDocumentModelEvent;
}

impl SingletonEntity for AIDocumentModel {}

impl AIDocumentModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn new_for_test() -> Self {
        Self
    }

    pub fn get_all_documents_for_conversation(
        &self,
        _conversation_id: AIConversationId,
    ) -> Vec<(AIDocumentId, AIDocument)> {
        vec![]
    }

    pub fn get_conversation_id_for_document_id(
        &self,
        _id: &AIDocumentId,
    ) -> Option<AIConversationId> {
        None
    }

    pub fn apply_persisted_content(
        &mut self,
        _id: AIDocumentId,
        _content: &str,
        _title: Option<&str>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }
}
