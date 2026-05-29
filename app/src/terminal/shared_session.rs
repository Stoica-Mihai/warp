//! Vestigial session-sharing leftovers.
//!
//! Warp's terminal session-sharing feature (sharing a live session over the
//! cloud for another user to join/view) has been stripped — the engine,
//! network, viewer/sharer, presence and UI machinery are gone. A few small
//! leaf types survive so the read-sites that gate terminal behaviour on
//! session role keep compiling: `SharedSessionStatus` collapses to a single
//! `NotShared` state (every predicate reports the no-sharing answer), and
//! `IsSharedSessionCreator` collapses to `No`. Removing those read-sites is
//! deferred.

use session_sharing_protocol::common::SessionId;
use session_sharing_protocol::sharer::SessionSourceType;

use crate::channel::ChannelState;

#[derive(Debug, Clone, Default)]
pub enum SharedSessionStatus {
    #[default]
    NotShared,
}

impl SharedSessionStatus {
    pub fn is_view_pending(&self) -> bool { false }
    pub fn is_active_viewer(&self) -> bool { false }
    pub fn is_finished_viewer(&self) -> bool { false }
    pub fn is_viewer(&self) -> bool { false }
    pub fn is_executor(&self) -> bool { false }
    pub fn is_reader(&self) -> bool { false }
    pub fn is_share_pending(&self) -> bool { false }
    pub fn is_active_sharer(&self) -> bool { false }
    pub fn is_sharer(&self) -> bool { false }
    pub fn is_sharer_or_viewer(&self) -> bool { false }
    pub fn as_keymap_context(&self) -> &'static str { "SharedSessionStatus_NotShared" }
}

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

pub fn join_native_intent(session_id: &SessionId) -> String {
    format!("{}://shared_session/{}", ChannelState::url_scheme(), session_id)
}

pub fn join_link(session_id: &SessionId) -> String {
    let use_web_url = !ChannelState::uses_staging_server() || cfg!(feature = "release_bundle");
    if use_web_url {
        format!("{}/session/{}", ChannelState::server_root_url(), session_id)
    } else {
        join_native_intent(session_id)
    }
}

use crate::terminal::model::terminal_model::BlockIndex;

/// Where a (now-removed) session-sharing action originated. Retained as a
/// plain data enum so the menu/footer construction sites keep compiling.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub enum SharedSessionActionSource {
    BlocklistContextMenu { block_index: Option<BlockIndex> },
    Tab,
    PaneHeader,
    CommandPalette,
    OnboardingBlock,
    Closed { is_confirm_close_session: bool },
    InactivityModal,
    NonUser,
    SharingDialog,
    RightClickMenu,
    FooterChip,
}

/// Scrollback selection for a (now-removed) shared session. Retained as a
/// plain data enum for the surviving construction sites.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedSessionScrollbackType {
    None,
    FromBlock { block_index: BlockIndex },
    All,
}
