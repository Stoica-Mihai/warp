mod context_menu;
pub mod editor;
pub mod file;
pub mod link;
mod styles;
pub mod telemetry;

use warpui::AppContext;

/// Initialize notebooks-related keybindings.
pub fn init(app: &mut AppContext) {
    self::file::init(app);
    self::editor::view::init(app);
}
