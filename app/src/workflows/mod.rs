
use serde::{Deserialize, Serialize};
use warp_core::context_flag::ContextFlag;
use warpui::AppContext;

pub mod categories;
use anyhow::Result;
use workflow::Workflow;

pub mod aliases;
pub mod command_parser;
pub mod info_box;
pub mod local_workflows;
pub mod workflow;
pub mod workflow_enum;
pub use categories::{CategoriesView, CategoriesViewEvent, WorkflowsViewAction};

use crate::drive::CloudObjectTypeAndId;
use crate::server::ids::{ServerId, SyncId};

pub fn init(app: &mut AppContext) {
    categories::init(app);
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum WorkflowSource {
    Global,
    Local,
    Project,
    PersonalCloud,
    WarpAI,

    /// A hardcoded workflow type that allows Warp to surface features as Workflows (e.g.
    /// a command to see our network log)
    App,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, PartialOrd)]
pub enum WorkflowSelectionSource {
    WarpDrive,
    CommandPalette,
    UniversalSearch,
    Voltron,
    WarpAI,
    Notebook,
    SlashMenu,
    UpArrowHistory,
    WorkflowView,
    AgentMode,
    Undefined,
    Alias,
}

#[derive(Debug, Clone, Copy)]
pub enum WorkflowViewMode {
    View,
    Edit,
    Create,
}

impl WorkflowViewMode {
    /// The editing mode supported for a workflow. Object sharing is not
    /// supported in this fork, so all workflows are editable.
    pub fn supported_edit_mode(_workflow_id: Option<SyncId>, _app: &AppContext) -> Self {
        Self::Edit
    }

    /// The viewing mode supported for this workflow.
    pub fn supported_view_mode(_workflow_id: Option<SyncId>, _app: &AppContext) -> Self {
        if ContextFlag::RunWorkflow.is_enabled() {
            Self::Edit
        } else {
            Self::View
        }
    }

}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WorkflowId(ServerId);
crate::server_id_traits! { WorkflowId, "Workflow" }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AIWorkflowOrigin {
    CommandSearch,
    AgentMode,
    LegacyWarpAI,
}

/// Wrapper type for a workflow that may be saved locally or using cloud sync.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowType {
    /// Saved workflows sourced from local, global, project, app collections, saved locally.
    Local(Workflow),
    /// Ephemeral/transient workflows created from Warp AI output
    AIGenerated {
        workflow: Workflow,
        origin: AIWorkflowOrigin,
    },
    /// A workflow that's part of a cloud notebook.
    Notebook(Workflow),
}

impl WorkflowType {
    pub fn as_workflow(&self) -> &Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated { workflow, .. } => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    /// Returns the contained [`Workflow`], consuming `self`.
    pub fn take_workflow(self) -> Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated { workflow, .. } => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    pub fn object_id(&self) -> Option<CloudObjectTypeAndId> {
        None
    }

    pub fn sync_id(&self) -> Option<SyncId> {
        None
    }

    pub fn server_id(&self) -> Option<WorkflowId> {
        None
    }

}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
