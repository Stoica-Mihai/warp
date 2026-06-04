pub mod styles {
    use pathfinder_color::ColorU;
    use warp_core::ui::theme::WarpTheme;

    pub fn default_border_color(theme: &WarpTheme) -> ColorU {
        theme.outline().into()
    }
}
