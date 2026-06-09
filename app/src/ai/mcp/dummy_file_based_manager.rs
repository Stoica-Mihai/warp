use std::path::PathBuf;

use uuid::Uuid;
use warpui::{Entity, ModelContext, SingletonEntity};

use super::MCPProvider;
use crate::ai::mcp::templatable_installation::TemplatableMCPServerInstallation;

pub struct FileBasedMCPManager {}

impl FileBasedMCPManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }

    pub fn file_based_servers(&self) -> Vec<&TemplatableMCPServerInstallation> {
        vec![]
    }

    pub fn get_installation_by_uuid(
        &self,
        _uuid: Uuid,
    ) -> Option<&TemplatableMCPServerInstallation> {
        None
    }

    pub fn directory_paths_for_installation_and_provider(
        &self,
        _uuid: Uuid,
        _provider: MCPProvider,
    ) -> Vec<PathBuf> {
        vec![]
    }
}

impl Entity for FileBasedMCPManager {
    type Event = ();
}

impl SingletonEntity for FileBasedMCPManager {}
