pub mod cloud_object_styling;
pub mod workflows;

use std::fmt;

pub use warp_server_client::drive::CloudObjectTypeAndId;

use crate::cloud_object::{ObjectType, Space};
use crate::server::ids::ServerId;
use crate::ui_components::icons::Icon;

pub const MIN_SIDEBAR_WIDTH: f32 = 250.;
pub const MAX_SIDEBAR_WIDTH_RATIO: f32 = 0.75;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct OpenWarpDriveObjectSettings {
    pub focused_folder_id: Option<ServerId>,
    pub invitee_email: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OpenWarpDriveObjectArgs {
    pub object_type: ObjectType,
    pub server_id: ServerId,
    pub settings: OpenWarpDriveObjectSettings,
}

#[derive(Copy, Clone, Debug)]
pub enum DriveObjectType {
    Workflow,
    AgentModeWorkflow,
    AIFact,
    AIFactCollection,
    Notebook {
        is_ai_document: bool,
    },
    Folder,
    MCPServer,
    MCPServerCollection,
}

impl From<DriveObjectType> for Icon {
    fn from(cloud_object_type: DriveObjectType) -> Icon {
        match cloud_object_type {
            DriveObjectType::Workflow => Icon::Workflow,
            DriveObjectType::AgentModeWorkflow => Icon::Prompt,
            DriveObjectType::AIFact => Icon::BookOpen,
            DriveObjectType::AIFactCollection => Icon::BookOpen,
            DriveObjectType::Notebook { is_ai_document } => {
                if is_ai_document {
                    Icon::Compass
                } else {
                    Icon::Notebook
                }
            }
            DriveObjectType::Folder => Icon::Folder,
            DriveObjectType::MCPServer => Icon::Dataflow,
            DriveObjectType::MCPServerCollection => Icon::Dataflow,
        }
    }
}

impl fmt::Display for DriveObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriveObjectType::Notebook { .. } => write!(f, "notebook"),
            DriveObjectType::Workflow => write!(f, "workflow"),
            DriveObjectType::Folder => write!(f, "folder"),
            DriveObjectType::AgentModeWorkflow => write!(f, "prompt"),
            DriveObjectType::AIFact => write!(f, "ai fact"),
            DriveObjectType::AIFactCollection => write!(f, "ai fact collection"),
            DriveObjectType::MCPServer => write!(f, "mcp server"),
            DriveObjectType::MCPServerCollection => write!(f, "mcp server collection"),
        }
    }
}

/// Uniquely identifies an item in Warp Drive index (includes spaces).
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum WarpDriveItemId {
    MCPServerCollection,
    Object(CloudObjectTypeAndId),
    Space(Space),
    Trash,
}
