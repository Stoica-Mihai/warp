//! This module houses all horizontal/cross-cutting AI functionality throughout
//! Warp (including Agent Mode).
pub mod execution_context;
pub(crate) mod agent;
pub mod ambient_agents;
pub(crate) mod artifact_download;
pub mod artifacts;
pub(crate) mod attachment_utils;
#[cfg(not(target_family = "wasm"))]
pub mod aws_credentials;
pub(crate) mod block_context;
pub(crate) mod blocklist;
pub(crate) mod conversation_navigation;
pub(crate) mod conversation_status_ui;
pub(crate) mod document;
pub(crate) mod get_relevant_files;
pub(crate) mod persisted_workspace;
pub mod request_usage_model;
pub(crate) mod skills;
pub(crate) mod voice;
pub use request_usage_model::*;
use warpui::AppContext;
pub mod cloud_agent_config;
pub mod cloud_environments;
pub mod execution_profiles;
pub mod connected_self_hosted_workers;
pub(crate) mod generate_block_title;
pub(crate) mod generate_code_review_content;
pub(crate) mod loading;
pub mod mcp;
pub mod outline;


pub fn init(app: &mut AppContext) {
    crate::terminal::view::keyboard_navigable_buttons::init(app);
    crate::terminal::view::toggleable_items::init(app);
}
