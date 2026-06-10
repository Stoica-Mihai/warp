//! Manages how we serialize blocklist AI data for persistence.
#![cfg_attr(not(feature = "local_fs"), allow(dead_code))]

use chrono::{DateTime, Local};

use crate::terminal::model::block::SerializedBlock;

/// The types of "blocks" we can store in our SQLite database for session restoration. Only command
/// blocks are true [`crate::terminal::model::block::Block`]s.
///
/// TODO(roland): now that there is no AI serialized block, consider removing this enum wrapper
#[derive(Debug, Clone, PartialEq)]
pub enum SerializedBlockListItem {
    Command { block: Box<SerializedBlock> },
}

impl SerializedBlockListItem {
    pub(crate) fn start_ts(&self) -> Option<DateTime<Local>> {
        match self {
            Self::Command { block } => block.start_ts,
        }
    }
}

impl From<crate::persistence::model::Block> for SerializedBlockListItem {
    fn from(value: crate::persistence::model::Block) -> Self {
        Self::Command {
            block: Box::new(SerializedBlock::from(value)),
        }
    }
}

impl From<SerializedBlock> for SerializedBlockListItem {
    fn from(value: SerializedBlock) -> Self {
        Self::Command {
            block: Box::new(value),
        }
    }
}
