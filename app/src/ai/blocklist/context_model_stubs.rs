use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::FairMutex;
use warpui::{AppContext, Entity, EntityId, ModelContext, ModelHandle};

use crate::ai::agent::conversation::{
    AIConversation, AIConversationAutoexecuteMode, AIConversationId, ConversationStatus,
};
use crate::ai::agent::todos::AIAgentTodoList;
use crate::ai::agent::{AIAgentAttachment, AIAgentContext, ImageContext};
use crate::ai::block_context::BlockContext;
use crate::ai::document::ai_document_model::AIDocumentId;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::session::Sessions;
use crate::terminal::model_events::ModelEventDispatcher;
use crate::terminal::view::agent_view_state::AgentViewEntryOrigin;
use crate::terminal::TerminalModel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingFile {
    pub file_name: String,
    pub file_path: PathBuf,
    pub mime_type: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentType {
    Image,
    File,
}

#[derive(Clone, Debug)]
pub enum PendingAttachment {
    Image(ImageContext),
    File(PendingFile),
}

impl PendingAttachment {
    pub fn file_name(&self) -> &str {
        match self {
            PendingAttachment::Image(img) => &img.file_name,
            PendingAttachment::File(file) => &file.file_name,
        }
    }

    pub fn attachment_type(&self) -> AttachmentType {
        match self {
            PendingAttachment::Image(_) => AttachmentType::Image,
            PendingAttachment::File(_) => AttachmentType::File,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PendingQueryState {
    Existing { conversation_id: AIConversationId },
    New {
        autoexecute_override: AIConversationAutoexecuteMode,
    },
}

impl Default for PendingQueryState {
    fn default() -> Self {
        Self::New {
            autoexecute_override: AIConversationAutoexecuteMode::default(),
        }
    }
}

impl PendingQueryState {
    pub fn targets_existing_conversation(&self) -> bool {
        matches!(self, PendingQueryState::Existing { .. })
    }
}

pub enum BlocklistAIContextEvent {
    UpdatedPendingContext {
        previous_block_ids: HashSet<BlockId>,
        requires_block_resync: bool,
        requires_text_resync: bool,
    },
    PendingQueryStateUpdated,
    QueueNextPromptToggled,
}

pub fn block_context_from_terminal_model(
    terminal_model: &TerminalModel,
    block_id: &BlockId,
    is_auto_attached: bool,
) -> Option<BlockContext> {
    let block = terminal_model
        .block_list()
        .block_index_for_id(block_id)
        .and_then(|block_id| terminal_model.block_list().block_at(block_id))?;

    let output = block.output_grid().content_summary(5000, 5000, false);

    Some(BlockContext {
        id: block_id.clone(),
        index: block.index(),
        command: block.command_to_string(),
        output,
        exit_code: block.exit_code(),
        is_auto_attached,
        started_ts: block.start_ts().cloned(),
        finished_ts: block.completed_ts().cloned(),
        pwd: None,
        shell: None,
        username: None,
        hostname: None,
        git_branch: None,
        os: None,
        session_id: None,
    })
}

pub struct BlocklistAIContextModel {
    pending_attachments: Vec<PendingAttachment>,
    pending_context_block_ids: HashSet<BlockId>,
    pending_query_state: PendingQueryState,
}

impl Entity for BlocklistAIContextModel {
    type Event = BlocklistAIContextEvent;
}

impl BlocklistAIContextModel {
    pub fn new(
        _sessions: ModelHandle<Sessions>,
        _model_event_dispatcher: &ModelHandle<ModelEventDispatcher>,
        _terminal_model: Arc<FairMutex<TerminalModel>>,
        _terminal_view_id: EntityId,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            pending_attachments: Vec::new(),
            pending_context_block_ids: HashSet::new(),
            pending_query_state: PendingQueryState::default(),
        }
    }

    pub fn has_locking_attachment(&self) -> bool {
        false
    }

    pub fn pending_context_block_ids(&self) -> &HashSet<BlockId> {
        &self.pending_context_block_ids
    }

    pub fn pending_context_selected_text(&self) -> Option<&String> {
        None
    }

    pub fn pending_attachments(&self) -> &[PendingAttachment] {
        &self.pending_attachments
    }

    pub fn pending_images(&self) -> Vec<&ImageContext> {
        vec![]
    }

    pub fn pending_files(&self) -> Vec<&PendingFile> {
        vec![]
    }

    pub fn transform_block_to_context(
        &self,
        _block_id: &BlockId,
        _is_auto_attached_in_agent_view: bool,
    ) -> Option<AIAgentContext> {
        None
    }

    pub fn pending_context(&self, _app: &AppContext, _is_user_query: bool) -> Vec<AIAgentContext> {
        vec![]
    }

    pub fn current_pwd(&self) -> Option<String> {
        None
    }

    pub fn home_directory(&self) -> Option<String> {
        None
    }

    pub fn update_directory_context(
        &mut self,
        _pwd: Option<String>,
        _home_dir: Option<String>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn set_pending_context_block_ids(
        &mut self,
        _ids: impl IntoIterator<Item = BlockId>,
        _requires_visual_resync: bool,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn set_pending_context_selected_text(
        &mut self,
        _text: Option<String>,
        _requires_visual_resync: bool,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn set_pending_document(
        &mut self,
        _document_id: Option<AIDocumentId>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn pending_document_id(&self) -> Option<AIDocumentId> {
        None
    }

    pub fn clear_pending_images(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn append_pending_images(
        &mut self,
        _images: Vec<ImageContext>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn append_pending_attachments(
        &mut self,
        _attachments: Vec<PendingAttachment>,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn remove_pending_image(&mut self, _index: usize, _ctx: &mut ModelContext<Self>) {}

    pub fn remove_last_pending_images(
        &mut self,
        _images_to_remove: usize,
        _ctx: &mut ModelContext<Self>,
    ) -> usize {
        0
    }

    pub fn remove_pending_attachment(&mut self, _index: usize, _ctx: &mut ModelContext<Self>) {}

    pub fn clear_pending_attachments(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn reset_context_to_default(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn pending_query_state(&self) -> &PendingQueryState {
        &self.pending_query_state
    }

    pub fn set_pending_query_state_for_existing_conversation(
        &mut self,
        _conversation_id: AIConversationId,
        _origin: AgentViewEntryOrigin,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn set_pending_query_state_for_new_conversation(
        &mut self,
        _origin: AgentViewEntryOrigin,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn can_start_new_conversation(&self) -> bool {
        true
    }

    pub fn selected_conversation_id(&self, _ctx: &AppContext) -> Option<AIConversationId> {
        None
    }

    pub fn selected_conversation<'a>(&self, _ctx: &'a AppContext) -> Option<&'a AIConversation> {
        None
    }

    pub fn selected_conversation_todolist<'a>(
        &self,
        _ctx: &'a AppContext,
    ) -> Option<&'a AIAgentTodoList> {
        None
    }

    pub fn pending_query_autoexecute_override(
        &self,
        _ctx: &AppContext,
    ) -> AIConversationAutoexecuteMode {
        AIConversationAutoexecuteMode::default()
    }

    pub fn is_queue_next_prompt_enabled(&self) -> bool {
        false
    }

    pub fn toggle_queue_next_prompt(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn toggle_pending_query_autoexecute(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn is_targeting_existing_conversation(&self) -> bool {
        false
    }

    pub fn selected_conversation_status_for_hint(
        &self,
        _app: &AppContext,
    ) -> Option<ConversationStatus> {
        None
    }

    pub fn can_attach_blocks(&self) -> bool {
        false
    }

    pub fn register_diff_hunk_attachment(
        &mut self,
        _diff_hunk_id: String,
        _attachment: AIAgentAttachment,
    ) {
    }

    pub fn get_diff_hunk_attachment(&self, _diff_hunk_id: &str) -> Option<&AIAgentAttachment> {
        None
    }

    pub fn clear_diff_hunk_attachments(&mut self) {}
}
