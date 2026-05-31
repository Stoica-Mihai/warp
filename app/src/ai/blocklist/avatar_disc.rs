//! Circular agent-avatar discs shared by usage and orchestration surfaces.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pathfinder_color::ColorU;
use warp_core::ui::theme::WarpTheme;
use warp_core::ui::Icon;
use warpui::elements::{
    ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Empty, Flex, MainAxisAlignment,
    MainAxisSize, ParentElement, Radius, Stack, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::{Element, SingletonEntity};

use crate::appearance::Appearance;

enum AvatarGlyph {
    Letter(char),
    Icon(Icon),
}

fn pill_palette(theme: &WarpTheme) -> [ColorU; 6] {
    [
        theme.ansi_fg_blue(),
        theme.ansi_fg_magenta(),
        theme.ansi_fg_cyan(),
        theme.ansi_fg_green(),
        theme.ansi_fg_yellow(),
        theme.ansi_fg_red(),
    ]
}

fn pill_avatar_color(name: &str, theme: &WarpTheme) -> ColorU {
    let palette = pill_palette(theme);
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let idx = (hasher.finish() as usize) % palette.len();
    palette[idx]
}

fn pill_initial(name: &str) -> char {
    name.trim()
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('A')
}

const TRANSCRIPT_AVATAR_SCALE: f32 = 1.25;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OrchestrationAvatar {
    Orchestrator,
    Agent { display_name: String },
}

impl OrchestrationAvatar {
    pub(crate) fn agent(display_name: String) -> Self {
        Self::Agent { display_name }
    }

    pub(crate) fn render(&self, app: &warpui::AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let size = app.font_cache().line_height(
            appearance.monospace_font_size(),
            appearance.line_height_ratio(),
        ) * TRANSCRIPT_AVATAR_SCALE;

        match self {
            Self::Orchestrator => render_orchestrator_avatar_disc(size, theme, appearance),
            Self::Agent { display_name } => {
                render_agent_avatar_disc(display_name, size, theme, appearance)
            }
        }
    }
}

pub(crate) fn render_orchestrator_avatar_disc(
    size: f32,
    theme: &WarpTheme,
    appearance: &Appearance,
) -> Box<dyn Element> {
    render_avatar_disc(
        theme.ansi_fg_cyan(),
        AvatarGlyph::Icon(Icon::Oz),
        size,
        theme,
        appearance,
    )
}

pub(crate) fn render_agent_avatar_disc(
    name: &str,
    size: f32,
    theme: &WarpTheme,
    appearance: &Appearance,
) -> Box<dyn Element> {
    render_avatar_disc(
        pill_avatar_color(name, theme),
        AvatarGlyph::Letter(pill_initial(name)),
        size,
        theme,
        appearance,
    )
}

fn render_avatar_disc(
    avatar_color: ColorU,
    glyph: AvatarGlyph,
    size: f32,
    theme: &WarpTheme,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let disc = ConstrainedBox::new(
        Container::new(Empty::new().finish())
            .with_background_color(avatar_color)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(size / 2.)))
            .finish(),
    )
    .with_width(size)
    .with_height(size)
    .finish();
    let glyph_size = size * 0.625;

    let glyph_element: Box<dyn Element> = match glyph {
        AvatarGlyph::Letter(letter) => {
            Text::new(letter.to_string(), appearance.ui_font_family(), glyph_size)
                .with_color(theme.background().into_solid())
                .with_style(Properties {
                    weight: Weight::Bold,
                    ..Default::default()
                })
                .finish()
        }
        AvatarGlyph::Icon(icon) => ConstrainedBox::new(icon.to_warpui_icon(theme.background()).finish())
            .with_width(glyph_size)
            .with_height(glyph_size)
            .finish(),
    };

    let glyph_centered = ConstrainedBox::new(
        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::Center)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(glyph_element)
                    .finish(),
            )
            .finish(),
    )
    .with_width(size)
    .with_height(size)
    .finish();

    Stack::new()
        .with_child(disc)
        .with_child(glyph_centered)
        .finish()
}
