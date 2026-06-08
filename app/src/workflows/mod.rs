
use serde::{Deserialize, Serialize};
use warp_core::context_flag::ContextFlag;
use warp_core::features::FeatureFlag;
use warpui::{AppContext, SingletonEntity};

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

use crate::cloud_object::model::view::CloudViewModel;
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
    /// The editing mode supported for a workflow.
    pub fn supported_edit_mode(workflow_id: Option<SyncId>, app: &AppContext) -> Self {
        let can_edit = workflow_id
            .map(|id| {
                CloudViewModel::as_ref(app)
                    .object_editability(&id.uid(), app)
                    .can_edit()
            })
            .unwrap_or(true);

        if !FeatureFlag::SharedWithMe.is_enabled() || can_edit {
            Self::Edit
        } else {
            Self::View
        }
    }

    /// The viewing mode supported for this workflow.
    pub fn supported_view_mode(workflow_id: Option<SyncId>, app: &AppContext) -> Self {
        let can_edit = workflow_id
            .map(|id| {
                CloudViewModel::as_ref(app)
                    .object_editability(&id.uid(), app)
                    .can_edit()
            })
            .unwrap_or(true);

        if FeatureFlag::SharedWithMe.is_enabled() && !can_edit {
            Self::View
        } else if ContextFlag::RunWorkflow.is_enabled() {
            Self::Edit
        } else {
            Self::View
        }
    }

    fn is_editable(&self) -> bool {
        match self {
            Self::View => false,
            Self::Edit | Self::Create => true,
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

    /// We don't show env var selection for Agent Mode suggested commands.
    pub(super) fn should_show_env_var_selection(&self) -> bool {
        !matches!(self, WorkflowType::AIGenerated { .. },)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
