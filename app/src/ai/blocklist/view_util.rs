//! This module contains common utilities for rendering Blocklist AI UI.
use std::sync::LazyLock;

use pathfinder_color::ColorU;
use warp_core::ui::appearance::Appearance;
use warpui::elements::{ConstrainedBox, Container};
use warpui::{AppContext, Element, SingletonEntity};

use crate::themes::theme::{AnsiColorIdentifier, Fill, WarpTheme};
use crate::ui_components::icons::Icon;

/// Text to use as a label throughout the app for user interactions that will attach selected
/// block(s) or text selections to a new AI query.
pub static ATTACH_AS_AGENT_MODE_CONTEXT_TEXT: LazyLock<&'static str> =
    LazyLock::new(|| "Attach as agent context");

/// Label we use for the the command palette action to create a new local Oz agent pane.
pub static NEW_AGENT_PANE_LABEL: LazyLock<&'static str> = LazyLock::new(|| "New Agent Pane");

/// Claude/Anthropic brand color (official brand orange #D97757).
/// Reference: https://github.com/anthropics/skills/blob/main/skills/brand-guidelines/SKILL.md
pub const CLAUDE_ORANGE: ColorU = ColorU {
    r: 0xD9,
    g: 0x77,
    b: 0x57,
    a: 0xFF,
};

/// Returns the color to be used for various AI signifiers
/// input with AI mode).
pub fn ai_brand_color(theme: &WarpTheme) -> ColorU {
    AnsiColorIdentifier::Magenta
        .to_ansi_color(&theme.terminal_colors().normal)
        .into()
}

/// Returns the AI icon element to be rendered in AI output blocks and the terminal input when in
/// AI mode. Takes a color parameter as the solid fill for the icon. We use [ai_brand_color] in most
/// cases.
pub fn render_ai_agent_mode_icon(app: &AppContext, color: impl Into<Fill>) -> Box<dyn Element> {
    render_input_icon(Icon::AgentMode, color.into(), app)
}

fn render_input_icon(icon: Icon, color: Fill, app: &AppContext) -> Box<dyn Element> {
    // Since the icon is rendered next to monospace text content, its size should scale to
    // based on the current font size -- specifically, its height must match the editor text line
    // height.
    let icon_size = ai_indicator_height(app);
    ConstrainedBox::new(
        Container::new(icon.to_warpui_icon(color).finish())
            .with_uniform_padding(icon_size / 8.)
            .finish(),
    )
    .with_width(icon_size)
    .with_height(icon_size)
    .finish()
}

/// Returns the size to be used for the AI icon in AI output blocks and the terminal input when in
/// AI mode.
///
/// This size is computed based on the user's current font size and line height ratio, such that the
/// size of the icon matches the user's text line height.  This is necessary because the AI icon in
/// the input is rendered next to text in the editor.
pub fn ai_indicator_height(app: &AppContext) -> f32 {
    let appearance = Appearance::as_ref(app);
    app.font_cache().line_height(
        appearance.monospace_font_size(),
        appearance.line_height_ratio(),
    )
}


