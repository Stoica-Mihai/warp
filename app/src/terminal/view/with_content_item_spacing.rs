use warpui::elements::Container;
use warpui::{AppContext, Element};

use crate::terminal::view::inline_action_icons::icon_size;

pub const CONTENT_HORIZONTAL_PADDING: f32 = 20.;
pub const CONTENT_ITEM_VERTICAL_MARGIN: f32 = 16.;

pub trait WithContentItemSpacing {
    fn with_content_item_spacing(self) -> Container;
    fn with_agent_output_item_spacing(self, app: &AppContext) -> Container;
}

impl WithContentItemSpacing for Box<dyn Element> {
    fn with_content_item_spacing(self) -> Container {
        Container::new(self)
            .with_horizontal_margin(CONTENT_HORIZONTAL_PADDING)
            .with_margin_bottom(CONTENT_ITEM_VERTICAL_MARGIN)
    }

    fn with_agent_output_item_spacing(self, app: &AppContext) -> Container {
        let left_margin = CONTENT_HORIZONTAL_PADDING + icon_size(app) + 16.;
        Container::new(self)
            .with_margin_left(left_margin)
            .with_margin_right(CONTENT_HORIZONTAL_PADDING)
            .with_margin_bottom(CONTENT_ITEM_VERTICAL_MARGIN)
    }
}
