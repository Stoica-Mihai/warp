//! WASM-only view functions for the Workspace.

use warp_core::channel::ChannelState;
use warpui::{ViewContext, ViewHandle};

use crate::uri::browser_url_handler::parse_current_url;
use crate::view_components::action_button::{ActionButton, NakedTheme, PrimaryTheme, SecondaryTheme};
use crate::ui_components::icons;
use crate::view_components::action_button::ButtonSize;
use crate::wasm_nux_dialog::{WasmNUXDialog, WasmNUXDialogEvent};
use crate::workspace::action::WorkspaceAction;
use crate::workspace::view::Workspace;

/// Builds the OZ runs URL for viewing all cloud runs.
fn build_oz_runs_url() -> String {
    format!("{}/runs", ChannelState::oz_root_url())
}

impl Workspace {
    pub(super) fn build_wasm_nux_dialog(ctx: &mut ViewContext<Self>) -> ViewHandle<WasmNUXDialog> {
        let wasm_nux_dialog = ctx.add_typed_action_view(|_| WasmNUXDialog::new());
        ctx.subscribe_to_view(&wasm_nux_dialog, |me, _, event, ctx| match event {
            WasmNUXDialogEvent::Close => {
                me.show_wasm_nux_dialog = false;
                ctx.notify();
            }
        });
        wasm_nux_dialog
    }

    pub(super) fn build_open_in_warp_button(
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<ActionButton> {
        ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("Open in Warp", PrimaryTheme).on_click(move |ctx| {
                // Get the current URL and dispatch action to open it on desktop
                if let Some(url) = parse_current_url() {
                    ctx.dispatch_typed_action(WorkspaceAction::OpenLinkOnDesktop(url));
                } else {
                    log::warn!("Could not get URL for Open in Warp button");
                }
            })
        })
    }

    pub(super) fn build_view_cloud_runs_button(
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<ActionButton> {
        let url = build_oz_runs_url();
        ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("View all cloud runs", SecondaryTheme).on_click(move |ctx| {
                ctx.dispatch_typed_action(WorkspaceAction::OpenLink(url.clone()));
            })
        })
    }

    pub(super) fn build_transcript_info_button(
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<ActionButton> {
        ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("", NakedTheme)
                .with_icon(icons::Icon::Info)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(
                        WorkspaceAction::ToggleConversationTranscriptDetailsPanel,
                    );
                })
        })
    }

}
