use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use strum_macros::{EnumDiscriminants, EnumIter};
use warp_core::features::FeatureFlag;

#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
#[derive(Clone, Serialize)]
pub enum CodebaseContextSyncType {
    Full,
    Initial,
    Incremental,
}

