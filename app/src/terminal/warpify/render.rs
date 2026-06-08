use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::theme::{Fill, WarpTheme};
use warpui::elements::{ConstrainedBox, Container, Flex, Icon, ParentElement, Rect, Stack, Text};
use warpui::fonts::FamilyId;
use warpui::{Element, PaintContext};

use super::SubshellSource;

/// The flag font size varies with the monospace font width, but if it gets too big it will start
/// to overlap with the prompt grid. This should eventually be fixed by growing the block height to
/// fit the flag, but for now we can limit the flag font size to this maximum value.
pub const MAXIMUM_FLAG_FONT_SIZE: f32 = 13.;

const SUBSHELL_FLAG_HORIZONTAL_PADDING: f32 = 8.;
const SUBSHELL_FLAG_VERTICAL_PADDING: f32 = 1.;

const ICON_MARGIN: f32 = 4.;
const TERMINAL_ICON: &str = "bundled/svg/terminal.svg";

/// Errored blocks have a red stripe, and subshells have a gray one.
pub const LEFT_STRIPE_WIDTH: f32 = 5.;


fn get_subshell_flag_info(subshell_source: &SubshellSource, theme: &WarpTheme) -> (String, Fill) {
    let SubshellSource::Command(command) = subshell_source;
    (command.to_string(), theme.subshell_background())
}

/// A single solid color vertical bar positioned on the left-hand side of a blocklist element
/// or the TextInput area, used to indicate being inside a context (like a subshell).
/// Implementation should match `[render_subshell_flag_pole]`.
pub fn draw_flag_pole(
    origin: Vector2F,
    height: f32,
    fill: impl Into<Fill>,
    ctx: &mut PaintContext,
) {
    ctx.scene
        .draw_rect_with_hit_recording(RectF::new(origin, Vector2F::new(LEFT_STRIPE_WIDTH, height)))
        .with_background(fill.into());
}

/// A single solid color vertical bar positioned on the left-hand side of a blocklist element
/// or the TextInput area, used to indicate being inside a context (like a subshell).
/// Implementation should match `[draw_subshell_flag_pole]`.
pub fn render_subshell_flag_pole(
    max_height: f32,
    fill: impl Into<warpui::elements::Fill>,
) -> Box<dyn Element> {
    ConstrainedBox::new(Rect::new().with_background(fill.into()).finish())
        .with_width(LEFT_STRIPE_WIDTH)
        .with_height(max_height)
        .finish()
}

/// This function creates the Element for the subshell flag, which may be needed by the block list
/// and the input editor.
pub fn render_subshell_flag(
    subshell_source: SubshellSource,
    font_family: FamilyId,
    font_size: f32,
    theme: &WarpTheme,
) -> Box<dyn Element> {
    let (flag_name, background_color) = get_subshell_flag_info(&subshell_source, theme);
    let container = Container::new(
        Flex::row()
            .with_children([
                render_icon(font_size - 2., theme.foreground()),
                Text::new_inline(flag_name, font_family, font_size - 2.)
                    .with_color(theme.foreground().into())
                    .finish(),
            ])
            .finish(),
    )
    .with_background(background_color)
    .with_padding_left(SUBSHELL_FLAG_HORIZONTAL_PADDING)
    .with_padding_right(SUBSHELL_FLAG_HORIZONTAL_PADDING)
    .with_padding_top(SUBSHELL_FLAG_VERTICAL_PADDING)
    .with_padding_bottom(SUBSHELL_FLAG_VERTICAL_PADDING)
    .finish();
    Stack::new().with_child(container).finish()
}

fn render_icon(font_size: f32, fill: Fill) -> Box<dyn Element> {
    Container::new(
        ConstrainedBox::new(Icon::new(TERMINAL_ICON, fill).finish())
            .with_max_width(font_size)
            .with_max_height(font_size)
            .finish(),
    )
    .with_margin_right(ICON_MARGIN)
    .finish()
}


