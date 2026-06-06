pub mod active_notebook_data;
mod context_menu;
pub mod editor;
pub mod file;
pub mod link;
pub mod manager;
pub mod notebook;
mod styles;
pub mod telemetry;


use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use warp_server_client::cloud_object::CloudObjectUpsertParams;
use warpui::AppContext;

use crate::ai::document::ai_document_model::AIDocumentId;
use crate::cloud_object::{
    CloudModelType,
    GenericCloudObject, ObjectType, Owner,
};
use crate::drive::CloudObjectTypeAndId;
use crate::persistence::ModelEvent;
use crate::server::ids::{ServerId, SyncId};

/// `CloudNotebook` is a notebook retrieved from the server.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CloudNotebookModel {
    pub title: String,
    pub data: String,
    pub ai_document_id: Option<AIDocumentId>,
    /// This is the server-generated conversation token, not the client-side AIConversationId.
    pub conversation_id: Option<String>,
}

pub type CloudNotebook = GenericCloudObject<NotebookId, CloudNotebookModel>;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl CloudModelType for CloudNotebookModel {
    type CloudObjectType = CloudNotebook;
    type IdType = NotebookId;

    fn model_type_name(&self) -> &'static str {
        if self.ai_document_id.is_some() {
            "Plan"
        } else {
            "Notebook"
        }
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Notebook
    }

    fn cloud_object_type_and_id(&self, id: SyncId) -> CloudObjectTypeAndId {
        CloudObjectTypeAndId::Notebook(id)
    }

    fn display_name(&self) -> String {
        self.title.clone()
    }

    fn set_display_name(&mut self, name: &str) {
        name.clone_into(&mut self.title);
    }

    fn upsert_event(params: CloudObjectUpsertParams<Self>) -> ModelEvent {
        ModelEvent::UpsertNotebook {
            notebook: CloudNotebook::from(params),
        }
    }

    fn bulk_upsert_event(objects: Vec<CloudObjectUpsertParams<Self>>) -> ModelEvent {
        ModelEvent::UpsertNotebooks(objects.into_iter().map(CloudNotebook::from).collect())
    }

    fn renders_in_warp_drive(&self) -> bool {
        true
    }

    fn can_export(&self) -> bool {
        true
    }

}

/// This is the notebook_id in the database associated with this notebook.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct NotebookId(ServerId);
crate::server_id_traits! { NotebookId, "Notebook" }

impl From<NotebookId> for SyncId {
    fn from(id: NotebookId) -> Self {
        Self::ServerId(id.into())
    }
}

/// A notebook location. Mainly, this lets us distinguish between cloud and file-based notebooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum NotebookLocation {
    /// A cloud notebook in the user's personal space.
    PersonalCloud,
    /// A cloud notebook in a team space.
    Team,
    /// A notebook backed by a local file.
    LocalFile,
    /// A notebook backed by a remote file.
    RemoteFile,
}

impl From<Owner> for NotebookLocation {
    fn from(owner: Owner) -> Self {
        // TODO(ben): Account for shared objects in notebook telemetry.
        match owner {
            Owner::User { .. } => NotebookLocation::PersonalCloud,
            Owner::Team { .. } => NotebookLocation::Team,
        }
    }
}

/// Initialize notebooks-related keybindings.
pub fn init(app: &mut AppContext) {
    self::notebook::init(app);
    self::file::init(app);
    self::editor::view::init(app);
}

/// Translate a notebook's Markdown content into an external Markdown format.
///
/// This:
/// * Normalizes code block languages
/// * Includes extra context for embedded objects.
#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
pub fn export_notebook(data: &str, ctx: &AppContext) -> anyhow::Result<String> {
    use warp_editor::content::buffer::Buffer;
    use warp_editor::content::markdown::MarkdownStyle;

    // Parse the Markdown directly rather than using [`Buffer::from_markdown`] so that we can
    // report errors to the exporter.
    let parsed = markdown_parser::parse_markdown(data)?;
    Ok(Buffer::export_to_markdown(
        parsed,
        Some(editor::notebook_embedded_item_conversion),
        MarkdownStyle::Export {
            app_context: Some(ctx),
            should_not_escape_markdown_punctuation: false,
        },
    ))
}
