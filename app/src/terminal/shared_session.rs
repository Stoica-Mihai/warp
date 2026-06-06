//! Vestigial session-sharing leftovers.
//!
//! Warp's terminal session-sharing (cloud feature) has been stripped. A few
//! plain data types survive for sites that haven't been fully cleaned up yet.

use session_sharing_protocol::sharer::SessionSourceType;

#[derive(Debug, Clone, Default)]
pub enum IsSharedSessionCreator {
    #[default]
    No,
}

#[derive(Debug, Clone)]
pub struct SharedSessionSource {
    pub source_type: SessionSourceType,
    pub source_task_id: Option<String>,
}

impl SharedSessionSource {
    pub fn user(source_task_id: Option<String>) -> Self {
        Self { source_type: SessionSourceType::User, source_task_id }
    }
    pub fn ambient_agent(task_id: Option<String>) -> Self {
        Self {
            source_type: SessionSourceType::AmbientAgent { task_id: task_id.clone() },
            source_task_id: task_id,
        }
    }
    pub fn orchestrator_task_id(&self) -> Option<&str> {
        self.source_task_id.as_deref().or(match &self.source_type {
            SessionSourceType::AmbientAgent { task_id } => task_id.as_deref(),
            SessionSourceType::User => None,
        })
    }
}

impl Default for SharedSessionSource {
    fn default() -> Self { Self::user(None) }
}
