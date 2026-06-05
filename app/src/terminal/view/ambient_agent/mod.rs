mod auth_secret_ftux_dropdown;
mod auth_secret_ftux_view;
pub(crate) mod auth_secret_selector;
mod block;
mod button_theme;
mod model;
mod progress_ui_state;
mod view_impl;

pub use auth_secret_ftux_view::{
    AuthSecretFtuxAction, AuthSecretFtuxView, AuthSecretFtuxViewEvent,
};
pub use auth_secret_selector::{
    AuthSecretSelector, AuthSecretSelectorAction, AuthSecretSelectorEvent,
};
pub use block::*;
pub use model::{AgentProgress, AmbientAgentViewModel, AmbientAgentViewModelEvent, Status};
use warpui::geometry::vector::Vector2F;
use warpui::{AppContext, ModelHandle, ViewHandle, WindowId};

use crate::pane_group::TerminalViewResources;
use crate::terminal::model::terminal_model::ConversationTranscriptViewerStatus;
use crate::terminal::shell::{ShellName, ShellType};
use crate::terminal::{
    MockTerminalManager, ShellLaunchState, TerminalManager, TerminalView,
};

/// Creates a cloud mode terminal view and manager for ambient agent sessions.
/// See `viewer::TerminalManager::enable_orchestration_polling` for the flag.
pub fn create_cloud_mode_view(
    resources: TerminalViewResources,
    view_bounds_size: Vector2F,
    window_id: WindowId,
    _enable_orchestration_polling: bool,
    ctx: &mut AppContext,
) -> (
    ViewHandle<TerminalView>,
    ModelHandle<Box<dyn TerminalManager>>,
) {
    // Session sharing was stripped; cloud-mode ambient agent panes now use a
    // local loading terminal manager (no cloud session join).
    let terminal_manager = MockTerminalManager::create_model(
        ShellLaunchState::ShellSpawned {
            available_shell: None,
            display_name: ShellName::blank(),
            shell_type: ShellType::Zsh,
        },
        resources,
        None,
        None,
        view_bounds_size,
        window_id,
        ctx,
    );
    terminal_manager.update(ctx, |terminal_manager, _ctx| {
        terminal_manager
            .model()
            .lock()
            .set_conversation_transcript_viewer_status(Some(
                ConversationTranscriptViewerStatus::Loading,
            ));
    });
    let terminal_view = terminal_manager.as_ref(ctx).view();
    (terminal_view, terminal_manager)
}

