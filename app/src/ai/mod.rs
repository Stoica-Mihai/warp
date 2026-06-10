//! This module houses all horizontal/cross-cutting AI functionality throughout
//! Warp (including Agent Mode).
pub mod execution_context;
pub(crate) mod agent_icons;
pub(crate) mod agent_types;
pub(crate) mod conversation_types;
#[cfg(not(target_family = "wasm"))]
pub(crate) mod blocklist;
pub(crate) mod conversation_navigation;
pub(crate) mod conversation_status_ui;
pub(crate) mod document;
pub(crate) mod persisted_workspace;
pub(crate) mod skills;
use warpui::AppContext;
pub mod execution_profiles;
pub(crate) mod loading;
pub mod mcp;
pub mod outline;


pub fn init(app: &mut AppContext) {
    crate::terminal::view::keyboard_navigable_buttons::init(app);
    crate::terminal::view::toggleable_items::init(app);
}
