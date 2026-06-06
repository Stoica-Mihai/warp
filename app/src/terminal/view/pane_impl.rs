//! This module contains the implementation of `BackingView` for `TerminalView`, as well as
//! business logic for integrating the terminal view with the pane infra (`crate::pane_group`).
use warpui::elements::{
    ConstrainedBox, CrossAxisAlignment, Flex, MainAxisAlignment, MainAxisSize,
    ParentElement, Shrinkable,
};
use warpui::prelude::Container;
use warpui::text_layout::ClipConfig;
use warpui::ui_components::components::UiComponent;
#[cfg(not(target_arch = "wasm32"))]
use warpui::{
    AppContext, Element, ModelHandle, SingletonEntity, TypedActionView, ViewContext,
    WeakModelHandle,
};

use super::{Event, PaneConfiguration, TerminalAction, TerminalViewState};
use crate::ai::conversation_types::ConversationStatus;
use crate::terminal::view::agent_view_state::agent_view_bg_fill;
use crate::appearance::Appearance;
use crate::features::FeatureFlag;
use crate::menu::{MenuItem, MenuItemFields};
use crate::pane_group::focus_state::{PaneFocusHandle, PaneGroupFocusEvent, PaneGroupFocusState};
use crate::pane_group::pane::view::header::components::{
    header_edge_min_width, render_pane_header_buttons, render_pane_header_title_text,
    render_three_column_header, CenteredHeaderEdgeWidth,
};
use crate::pane_group::pane::view::header::render_pane_header_draggable;
use crate::pane_group::pane::view::PaneHeaderAction;
use crate::pane_group::pane::{view, PaneStack};
use crate::pane_group::{BackingView, SplitPaneState};
use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;
use crate::terminal::{TerminalManager, TerminalView};
use crate::ui_components::agent_icon::terminal_view_agent_icon_variant;
use crate::ui_components::buttons::icon_button_with_color;
use crate::ui_components::icon_with_status::render_icon_with_status;
use crate::ui_components::{blended_colors, icons};
use crate::workspace::tab_settings::TabSettings;

/// Total size of the agent icon-with-status component rendered in the pane header.
/// Sub-components (circle, badge, cloud) are derived inside `render_icon_with_status`.
/// Sized so the component fits comfortably within `PANE_HEADER_HEIGHT` (34px) with a
/// few pixels of vertical buffer.
const PANE_HEADER_AGENT_SIZE: f32 = 26.;

impl TerminalView {
    /// Returns a reference to the focus handle if one has been set.
    pub fn focus_handle(&self) -> Option<&PaneFocusHandle> {
        self.focus_handle.as_ref()
    }

    fn handle_focus_state_event(
        &mut self,
        _focus_state: ModelHandle<PaneGroupFocusState>,
        event: &PaneGroupFocusEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(focus_handle) = &self.focus_handle else {
            return;
        };

        if focus_handle.is_affected(event) {
            self.on_pane_state_change(ctx);
        }
    }

    /// Set the pane configuration for this terminal view.
    pub fn set_pane_configuration(&mut self, pane_configuration: ModelHandle<PaneConfiguration>) {
        self.pane_configuration = pane_configuration;
    }

    /// Respond to changes to the active session or split pane states.
    pub fn on_pane_state_change(&mut self, ctx: &mut ViewContext<Self>) {
        self.refresh_pane_header(ctx);

        // Trigger refresh of the pane header overflow menu to reflect the new pane state
        // (e.g., updating the Maximize/Minimize pane menu item)
        self.pane_configuration.update(ctx, |config, ctx| {
            config.refresh_pane_header_overflow_menu_items(ctx);
        });

        if !self.is_pane_focused(ctx) {
            // Don't need to call ctx.notify here as clear_selected_blocks already
            // calls ctx.notify internally
            self.clear_selected_blocks(ctx);
            self.clear_selected_text(ctx);
        } else {
            ctx.notify();
        }
    }

    pub fn refresh_pane_header(&mut self, ctx: &mut ViewContext<Self>) {
        let is_active_session = self.is_active_session(ctx);
        self.pane_configuration
            .update(ctx, move |pane_config, ctx| {
                pane_config.set_show_active_pane_indicator(is_active_session, ctx);
                pane_config.refresh_pane_header_overflow_menu_items(ctx);
            });
    }

    /// Set the pane title from agent chrome when available, falling back to the regular terminal title.
    pub(super) fn update_pane_configuration(&mut self, ctx: &mut ViewContext<Self>) {
        let is_ambient_agent = self.is_ambient_agent_session(ctx);
        let selected_conversation_title = self.selected_conversation_display_title(ctx);
        let selected_cli_agent_title = self.selected_cli_agent_title_for_chrome(ctx);

        // Prefer CLI agent session text before the terminal title,
        // matching the vertical-tab behavior in terminal_primary_line_data().
        let new_pane_title = if let Some(cli_agent_title) = selected_cli_agent_title {
            self.is_using_conversation_for_pane_header_title = false;
            cli_agent_title
        } else if self.is_long_running_and_user_controlled() && !self.terminal_title.is_empty() {
            self.is_using_conversation_for_pane_header_title = false;
            self.terminal_title.clone()
        } else {
            match selected_conversation_title {
                Some(conversation_title) => {
                    self.is_using_conversation_for_pane_header_title = true;
                    conversation_title
                }
                None => {
                    if is_ambient_agent {
                        default_agent_conversation_title(is_ambient_agent)
                    } else {
                        self.terminal_title.clone()
                    }
                }
            }
        };
        self.pane_configuration.update(ctx, |pane_config, ctx| {
            pane_config.set_title(new_pane_title, ctx);
            pane_config.notify_header_content_changed(ctx);
        });
        self.update_agent_view_pane_header(ctx);
    }

    /// Updates the pane header's shareable object based on agent view state.
    /// This should be called when entering/exiting agent view or when the conversation changes.
    pub(super) fn update_agent_view_pane_header(&mut self, _ctx: &mut ViewContext<Self>) {}

    pub(super) fn is_pane_focused(&self, app: &AppContext) -> bool {
        self.focus_handle.as_ref().is_none_or(|h| h.is_focused(app))
    }

    pub fn is_active_session(&self, app: &AppContext) -> bool {
        self.focus_handle
            .as_ref()
            .is_some_and(|h| h.is_active_session(app))
    }

    pub(super) fn split_pane_state(&self, app: &AppContext) -> SplitPaneState {
        self.focus_handle
            .as_ref()
            .map_or(SplitPaneState::NotInSplitPane, |h| h.split_pane_state(app))
    }

    /// Renders the back button for the pane header, or an empty element if the
    /// back button should not be shown.
    fn maybe_render_header_back_button(&self, _app: &AppContext) -> Box<dyn Element> {
        Flex::row().finish()
    }

    fn render_header_title(
        &self,
        is_fullscreen_agent_view: bool,
        header_ctx: &view::HeaderRenderContext,
        app: &AppContext,
    ) -> Box<dyn Element> {
        // V2 swap-panes semantics: every conversation in the orchestration
        // tree (orchestrator + each child) gets the orchestration pill bar
        // rendered above the agent view header, so the pane title here
        // falls back to the regular conversation title. Breadcrumbs used
        // to render here for split-off child views, but the swap-panes
        // refactor removed the split-off code path — the pill bar is now
        // shown on every view, so a breadcrumb row alongside it would
        // double-render the same navigation affordance.

        let appearance = Appearance::as_ref(app);
        let pane_config = self.pane_configuration.as_ref(app);
        let title = pane_config.title().to_owned();
        let clip_config = if self.is_using_conversation_for_pane_header_title {
            ClipConfig::ellipsis()
        } else {
            ClipConfig::start()
        };

        let should_render_ambient_agent_indicator =
            self.ambient_agent_task_id_for_details_panel(app).is_some()
                || self.model.lock().is_shared_ambient_agent_session();
        let theme = appearance.theme();
        let render_agent_circle = |variant| {
            render_icon_with_status(
                variant,
                PANE_HEADER_AGENT_SIZE,
                0.,
                theme,
                theme.background(),
            )
        };
        let pane_indicator = if should_render_ambient_agent_indicator {
            // Shared/viewed ambient session: route through the shared helper so the pane header
            // renders the same brand-color circle + cloud lobe + status as the vertical tab.
            terminal_view_agent_icon_variant(self, app).map(render_agent_circle)
        } else if self.is_using_conversation_for_pane_header_title
        {
            // Conversation-bound terminal: same shared helper — produces an OzAgent variant for
            // local conversations and a CLIAgent variant for the (rare) CLI-backed terminal.
            terminal_view_agent_icon_variant(self, app).map(render_agent_circle)
        } else {
            self.render_terminal_mode_indicator(app)
        };

        let is_pane_dragging = header_ctx.draggable_state.is_dragging();
        let mut center_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Min);
        if let Some(indicator) = pane_indicator {
            center_row.add_child(Container::new(indicator).with_margin_right(4.).finish());
        }
        let title_text = render_pane_header_title_text(title, appearance, clip_config);
        if is_pane_dragging {
            // During drag, all children must be non-flex to avoid panics
            // from infinite constraints on flex children.
            center_row.add_child(title_text);
        } else {
            let title_element =
                if is_fullscreen_agent_view && self.is_using_conversation_for_pane_header_title {
                    Shrinkable::new(
                        1.0,
                        ConstrainedBox::new(title_text)
                            .with_max_width(400.0)
                            .finish(),
                    )
                    .finish()
                } else {
                    Shrinkable::new(1.0, title_text).finish()
                };
            center_row.add_child(title_element);
        }

        center_row.finish()
    }

    /// Returns the right-column element and the estimated minimum width of
    /// the right-column content (used to set the edge width for centering).
    fn render_header_actions(
        &self,
        header_ctx: &view::HeaderRenderContext,
        app: &AppContext,
    ) -> (Box<dyn Element>, f32) {
        let appearance = Appearance::as_ref(app);
        let is_fullscreen_agent_view = false;
        let icon_color = Some(
            appearance
                .theme()
                .sub_text_color(appearance.theme().background()),
        );
        let button_size = if is_fullscreen_agent_view {
            Some(24.0)
        } else {
            None
        };

        let mut left_of_overflow: Option<Box<dyn Element>> = None;

        let mut icon_button_count: u32 = 0;

        // Cloud-mode-only ambient agent cancel button is shown while we're waiting
        // for the session to be ready.
        let is_waiting_for_session = false;
        let button_element = if is_waiting_for_session {
            Some(self.render_ambient_agent_cancel_button(app))
        } else {
            None
        };

        if let Some(button) = button_element {
            icon_button_count += 1;
            if let Some(existing) = left_of_overflow {
                left_of_overflow =
                    Some(Flex::row().with_child(existing).with_child(button).finish());
            } else {
                left_of_overflow = Some(button);
            }
        }

        let mut right_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Min);
        if let Some(content) = left_of_overflow {
            right_row.add_child(content);
        }
        let show_close_button = self
            .focus_handle
            .as_ref()
            .is_some_and(|h| h.is_in_split_pane(app));
        right_row.add_child(
            render_pane_header_buttons::<TerminalAction, TerminalAction>(
                header_ctx,
                appearance,
                show_close_button,
                icon_color,
                button_size,
            ),
        );
        icon_button_count += show_close_button as u32 + header_ctx.has_overflow_items as u32;

        let min_width = header_edge_min_width(icon_button_count);
        (right_row.finish(), min_width)
    }

    fn render_parent_conversation_header_card(
        &self,
        _app: &AppContext,
    ) -> Option<Box<dyn Element>> {
        None
    }

    fn maybe_add_parent_navigation_card(
        &self,
        header: Box<dyn Element>,
        parent_conversation_header_card: Option<Box<dyn Element>>,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        if !FeatureFlag::OrchestrationV2.is_enabled() {
            return header;
        }

        if let Some(parent_card) = parent_conversation_header_card {
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Container::new(parent_card)
                        .with_padding_left(4.)
                        .with_padding_right(4.)
                        .with_padding_top(4.)
                        .with_padding_bottom(2.)
                        .finish(),
                )
                .with_child(header)
                .finish()
        } else {
            header
        }
    }

    fn render_terminal_pane_header(
        &self,
        header_ctx: &view::HeaderRenderContext,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_fullscreen_agent_view = false;
        let parent_conversation_header_card = self.render_parent_conversation_header_card(app);

        let left = self.maybe_render_header_back_button(app);
        let center = self.render_header_title(is_fullscreen_agent_view, header_ctx, app);
        let (right, min_actions_width) = self.render_header_actions(header_ctx, app);

        let header = render_three_column_header(
            left,
            center,
            right,
            CenteredHeaderEdgeWidth {
                min: min_actions_width,
                max: 200.0,
            },
            header_ctx.header_left_inset,
            header_ctx.draggable_state.is_dragging(),
        );
        // Make only the title row draggable; the secondary row (pill
        // bar / breadcrumbs / navigation card) sits outside the drag
        // region so its own mouse-driven widgets (notably the pill
        // bar's scrollbar thumb) keep their hit-targets.
        let draggable_header = render_pane_header_draggable::<TerminalView>(
            self.pane_configuration.clone(),
            header,
            header_ctx.draggable_state.clone(),
            app,
        );
        let header = self.maybe_add_parent_navigation_card(
            draggable_header,
            parent_conversation_header_card,
            app,
        );

        if is_fullscreen_agent_view {
            Container::new(header)
                .with_background(agent_view_bg_fill(app))
                .finish()
        } else {
            header
        }
    }
}

impl BackingView for TerminalView {
    type PaneHeaderOverflowMenuAction = TerminalAction;
    type CustomAction = TerminalAction;
    type AssociatedData = ModelHandle<Box<dyn TerminalManager>>;

    fn set_pane_stack(
        &mut self,
        pane_stack: WeakModelHandle<PaneStack<Self>>,
        _ctx: &mut ViewContext<Self>,
    ) {
        self.pane_stack = Some(pane_stack);
    }

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn handle_custom_action(&mut self, action: &Self::CustomAction, ctx: &mut ViewContext<Self>) {
        self.handle_action(action, ctx);
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::CloseRequested);
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.redetermine_global_focus(ctx);
    }

    fn on_pane_header_overflow_menu_toggled(&mut self, _is_open: bool, _ctx: &mut ViewContext<Self>) {
    }

    fn pane_header_overflow_menu_items(
        &self,
        ctx: &AppContext,
    ) -> Vec<MenuItem<Self::PaneHeaderOverflowMenuAction>> {
        let _model = self.model.lock();
        let mut items = vec![];

        // Split-pane related items.
        if self.split_pane_state(ctx).is_in_split_pane() {
            if !items.is_empty() {
                items.push(MenuItem::Separator);
            }

            let is_maximized = self.split_pane_state(ctx).is_maximized();
            items.push(
                MenuItemFields::toggle_pane_action(is_maximized)
                    .with_on_select_action(TerminalAction::ToggleMaximizePane)
                    .into_item(),
            );
        }

        items
    }

    fn should_render_header(&self, _app: &AppContext) -> bool {
        false
    }

    fn render_header_content(
        &self,
        header_ctx: &view::HeaderRenderContext,
        app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::Custom {
            element: self.render_terminal_pane_header(header_ctx, app),
            // We wrap only the title row in the drag handler ourselves;
            // the secondary row stays interactive.
            has_custom_draggable_behavior: true,
        }
    }

    /// Sets the focus handle for this terminal view, enabling it to track its split pane state.
    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle.clone());
        // Subscribe to focus state changes to update pane state when focus/split state changes
        ctx.subscribe_to_model(
            focus_handle.focus_state_handle(),
            Self::handle_focus_state_event,
        );
        self.input.update(ctx, |input, ctx| {
            input.set_focus_handle(focus_handle, ctx);
        });
        self.on_pane_state_change(ctx);
    }
}

impl TerminalView {
    /// Render the cancel button for cancelling the ambient agent task while it's loading.
    fn render_ambient_agent_cancel_button(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder().clone();

        icon_button_with_color(
            appearance,
            icons::Icon::StopFilled,
            false, /* active */
            self.ambient_agent_cancel_mouse_state.clone(),
            blended_colors::text_sub(theme, theme.background()).into(),
        )
        .with_tooltip(move || ui_builder.tool_tip("Cancel".to_string()).build().finish())
        .build()
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action::<PaneHeaderAction<TerminalAction, TerminalAction>>(
                PaneHeaderAction::CustomAction(TerminalAction::CancelAmbientAgentTask),
            );
        })
        .finish()
    }


    /// Render the indicator for terminal mode (no conversation selected).
    /// Shows error indicator if terminal is in error state, otherwise shell indicator on Windows.
    fn render_terminal_mode_indicator(&self, app: &AppContext) -> Option<Box<dyn Element>> {
        let appearance = Appearance::as_ref(app);
        let font_size = appearance.ui_font_size();

        // Error indicator takes priority
        if matches!(self.current_state.state, TerminalViewState::Errored) {
            return Some(
                ConstrainedBox::new(
                    icons::Icon::AlertTriangle
                        .to_warpui_icon(appearance.theme().ui_error_color().into())
                        .finish(),
                )
                .with_height(font_size)
                .with_width(font_size)
                .finish(),
            );
        }

        // Shell indicator (Windows only)
        if let Some(shell_indicator_type) = self.shell_indicator_type {
            let shell_indicator_icon = shell_indicator_type
                .to_icon()
                .to_warpui_icon(
                    blended_colors::text_sub(appearance.theme(), appearance.theme().background())
                        .into(),
                )
                .finish();
            return Some(
                ConstrainedBox::new(shell_indicator_icon)
                    .with_height(font_size)
                    .with_width(font_size)
                    .finish(),
            );
        }

        None
    }

    /// Render shared session header content (participant avatars and role controls).

    pub fn is_ambient_agent_session(&self, _ctx: &AppContext) -> bool {
        false
    }

    /// Returns `true` while a cloud-mode ambient agent run is still spinning up. This covers
    /// both the `WaitingForSession` phase (env being provisioned, "Connecting to Host") and
    /// the post-session pre-first-exchange phase (session ready, harness not started, no
    /// exchange yet). In either case the run is committed and we want the UI to read as busy.
    fn is_in_cloud_agent_setup_phase(&self, _ctx: &AppContext) -> bool {
        false
    }

    /// Selected conversation status for chrome. Warp AI conversations are stripped, so this only
    /// surfaces [`ConversationStatus::InProgress`] while the active block is long-running or a
    /// cloud-mode ambient agent is still in its environment-setup phase.
    pub fn selected_conversation_status(&self, ctx: &AppContext) -> Option<ConversationStatus> {
        let long_running = self.is_long_running();
        let cloud_setup = self.is_in_cloud_agent_setup_phase(ctx);
        if (long_running || cloud_setup) && self.is_ambient_agent_session(ctx) {
            return Some(ConversationStatus::InProgress);
        }
        None
    }

    pub fn selected_conversation_status_for_display(
        &self,
        ctx: &AppContext,
    ) -> Option<ConversationStatus> {
        self.selected_conversation_status(ctx)
    }

    pub fn selected_conversation_display_title(&self, _ctx: &AppContext) -> Option<String> {
        None
    }

    pub fn selected_conversation_latest_user_prompt_for_tab_name(
        &self,
        _ctx: &AppContext,
    ) -> Option<String> {
        None
    }

    fn selected_cli_agent_title_for_chrome(&self, ctx: &AppContext) -> Option<String> {
        let session = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .filter(|session| session.listener.is_some())?;

        if *TabSettings::as_ref(ctx).use_latest_user_prompt_as_conversation_title_in_tab_names {
            session
                .session_context
                .latest_user_prompt()
                .or_else(|| session.session_context.title_like_text())
        } else {
            session.session_context.title_like_text()
        }
    }
}

fn default_agent_conversation_title(is_ambient_agent: bool) -> String {
    if is_ambient_agent {
        "New cloud agent".to_owned()
    } else {
        "New agent conversation".to_owned()
    }
}
