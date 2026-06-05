//! Button theme for ambient-agent selector buttons. Relocated from the removed agent_view
//! footer module since the ambient model/harness selectors still render with it.
use pathfinder_color::ColorU;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::theme::Fill;

use crate::view_components::action_button::ActionButtonTheme;

pub(crate) struct NakedHeaderButtonTheme;

impl ActionButtonTheme for NakedHeaderButtonTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<Fill> {
        if hovered {
            Some(internal_colors::fg_overlay_1(appearance.theme()))
        } else {
            None
        }
    }

    fn text_color(
        &self,
        _hovered: bool,
        _background: Option<Fill>,
        appearance: &Appearance,
    ) -> ColorU {
        appearance
            .theme()
            .sub_text_color(appearance.theme().surface_1())
            .into_solid()
    }

    fn border(&self, _appearance: &Appearance) -> Option<ColorU> {
        None
    }

    fn should_opt_out_of_contrast_adjustment(&self) -> bool {
        true
    }

    fn font_properties(&self) -> Option<warpui::fonts::Properties> {
        None
    }
}

