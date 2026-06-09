use std::collections::HashMap;

use markdown_parser::html_parser::WARP_EMBED_ATTRIBUTE_NAME;
use serde_yaml::Mapping;
use warp_editor::content::markdown::MarkdownStyle;
use warp_editor::render::layout::TextLayout;
use warp_editor::render::model::{
    BlockSpacing, BrokenBlockEmbedding, EmbeddedItem, EmbeddedItemHTMLRepresentation,
    EmbeddedItemRichFormat, LaidOutEmbeddedItem,
};
use warp_editor::render::BLOCK_FOOTER_HEIGHT;
use warpui::elements::{Margin, Padding};
use warpui::AppContext;

// Spacing for the text sections (e.g. title, command) within the workflow card.
const EMBED_WORKFLOW_TEXT_SPACING: BlockSpacing = BlockSpacing {
    margin: Margin::uniform(0.)
        .with_top(8.)
        .with_left(4.)
        .with_bottom(8.)
        .with_right(16.),
    padding: Padding::uniform(8.)
        .with_left(40.)
        .with_top(16.)
        // Reserve space for the buttons.
        .with_bottom(BLOCK_FOOTER_HEIGHT),
};
#[derive(Debug)]
pub struct EmbeddedWorkflow {
    hashed_id: String,
}

impl EmbeddedWorkflow {
    pub fn new(hashed_id: String) -> Self {
        Self { hashed_id }
    }
}

impl EmbeddedItem for EmbeddedWorkflow {
    fn layout(&self, text_layout: &TextLayout, _app: &AppContext) -> Box<dyn LaidOutEmbeddedItem> {
        let base_text_style = &text_layout.rich_text_styles().base_text;
        let width = text_layout.max_width() - EMBED_WORKFLOW_TEXT_SPACING.x_axis_offset();
        Box::new(BrokenBlockEmbedding::new(width, base_text_style.font_size))
    }

    fn hashed_id(&self) -> &str {
        self.hashed_id.as_str()
    }

    fn to_mapping(&self, _style: MarkdownStyle) -> Mapping {
        let mut base: Mapping = Default::default();
        base.insert("id".into(), self.hashed_id().into());
        base
    }

    fn to_rich_format(&self, _app: &AppContext) -> EmbeddedItemRichFormat<'_> {
        EmbeddedItemRichFormat {
            plain_text: "".to_owned(),
            html: EmbeddedItemHTMLRepresentation {
                element_name: "pre",
                content: "".to_owned(),
                attributes: HashMap::from([(WARP_EMBED_ATTRIBUTE_NAME, self.hashed_id())]),
            },
        }
    }
}


