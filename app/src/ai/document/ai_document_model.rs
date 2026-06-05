pub use ai::document::{AIDocumentId, AIDocumentVersion};

use chrono::{DateTime, Local};
use warpui::{Entity, ModelContext, SingletonEntity};

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

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self
    }

    pub fn get_all_documents_for_conversation(
        &self,
        _conversation_id: AIConversationId,
    ) -> Vec<(AIDocumentId, AIDocument)> {
        vec![]
    }


}
