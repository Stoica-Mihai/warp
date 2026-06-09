use std::borrow::Cow;
use std::mem;
use std::ops::Range;
use std::sync::Arc;

use string_offset::{ByteOffset, CharOffset};
use warp_editor::content::anchor::Anchor;
use warp_editor::content::buffer::Buffer;
use warp_editor::content::selection_model::BufferSelectionModel;
use warp_editor::editor::EmbeddedItemModel;
use warpui::elements::{Border, Container, MouseStateHandle};
use warpui::platform::Cursor;
use warpui::ui_components::button::ButtonVariant;
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Element, Entity, ModelAsRef, ModelContext, ModelHandle, SingletonEntity};

use super::embedded_item::EmbeddedWorkflow;
use super::model::ChildModelHandle;
use super::view::EditorViewAction;
use super::NotebookWorkflow;
use crate::appearance::Appearance;
use crate::themes::theme::AnsiColorIdentifier;

#[derive(Default)]
struct MouseStateHandles {
    remove_embedding_button_state: MouseStateHandle,
}

pub struct NotebookEmbed {
    start: Anchor,
    hashed_id: String,
    is_selected: bool,
    content: ModelHandle<Buffer>,
    selection_model: ModelHandle<BufferSelectionModel>,
    mouse_state_handles: MouseStateHandles,
    cached_syntax_color: Option<Vec<(Range<ByteOffset>, AnsiColorIdentifier)>>,
}

impl NotebookEmbed {
    pub fn new(
        start: CharOffset,
        hashed_id: String,
        content: ModelHandle<Buffer>,
        selection_model: ModelHandle<BufferSelectionModel>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let start = selection_model.update(ctx, |selection_model, ctx| {
            selection_model.anchor(start, ctx)
        });

        let embedding = Self {
            start,
            hashed_id,
            content,
            selection_model,
            is_selected: false,
            mouse_state_handles: Default::default(),
            cached_syntax_color: None,
        };

        embedding.highlight_syntax(ctx);
        embedding
    }

    pub fn highlight_syntax(&self, _ctx: &mut ModelContext<Self>) {}

    pub fn try_apply_cached_highlighting(&self, ctx: &mut ModelContext<Self>) {
        if self.cached_syntax_color.is_some() {
            self.update_buffer_with_syntax_color(ctx);
        }
    }

    fn update_buffer_with_syntax_color(&self, ctx: &mut ModelContext<Self>) {
        let Some(offset) = self.start_offset(ctx) else {
            return;
        };
        self.content.update(ctx, |buffer, ctx| {
            buffer.replace_embedding_at_offset(
                offset,
                Arc::new(EmbeddedWorkflow::new(self.hashed_id.clone())),
                self.selection_model.clone(),
                ctx,
            )
        });
    }

    pub fn hashed_id(&self) -> &str {
        self.hashed_id.as_str()
    }

    pub fn refresh_item_state(&self, ctx: &mut ModelContext<Self>) {
        let Some(offset) = self.start_offset(ctx) else {
            return;
        };

        self.content.update(ctx, |buffer, ctx| {
            buffer.replace_embedding_at_offset(
                offset,
                Arc::new(EmbeddedWorkflow::new(self.hashed_id.clone())),
                self.selection_model.clone(),
                ctx,
            )
        });

        // Re-highlight syntax since the command might have changed.
        self.highlight_syntax(ctx);
    }

    pub fn start_offset(&self, ctx: &impl ModelAsRef) -> Option<CharOffset> {
        self.selection_model.as_ref(ctx).resolve_anchor(&self.start)
    }

    fn selectable(&self, _ctx: &AppContext) -> bool {
        false
    }
}

impl Entity for NotebookEmbed {
    type Event = ();
}

impl EmbeddedItemModel for NotebookEmbed {
    fn render_item_footer(&self, _ctx: &AppContext) -> Option<Box<dyn Element>> {
        None
    }

    fn border(&self, app: &AppContext) -> Option<Border> {
        if self.is_selected {
            let border_fill = Appearance::as_ref(app).theme().accent();
            Some(Border::all(3.).with_border_fill(border_fill))
        } else {
            None
        }
    }

    fn render_remove_embedding_button(&self, ctx: &AppContext) -> Option<Box<dyn Element>> {
        let appearance = Appearance::as_ref(ctx);
        let offset = self.start_offset(ctx)?;
        Some(
            Container::new(
                appearance
                    .ui_builder()
                    .button(
                        ButtonVariant::Text,
                        self.mouse_state_handles
                            .remove_embedding_button_state
                            .clone(),
                    )
                    .with_text_label("Remove".to_string())
                    .build()
                    .with_cursor(Cursor::Arrow)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(EditorViewAction::RemoveEmbeddingAt(offset));
                    })
                    .finish(),
            )
            .with_margin_right(12.)
            .finish(),
        )
    }
}

impl ChildModelHandle for ModelHandle<NotebookEmbed> {
    fn start_offset(&self, app: &AppContext) -> Option<CharOffset> {
        self.as_ref(app).start_offset(app)
    }

    fn end_offset(&self, app: &AppContext) -> Option<CharOffset> {
        // Embedding should always take one character offset.
        self.as_ref(app).start_offset(app).map(|offset| offset + 1)
    }

    fn selectable(&self, app: &AppContext) -> bool {
        self.as_ref(app).selectable(app)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn executable_workflow(&self, _app: &AppContext) -> Option<NotebookWorkflow> {
        None
    }

    fn executable_command<'a>(&'a self, _app: &'a AppContext) -> Option<Cow<'a, str>> {
        None
    }

    fn selected(&self, app: &AppContext) -> bool {
        self.as_ref(app).is_selected
    }

    fn set_selected(&self, selected: bool, ctx: &mut AppContext) -> bool {
        self.update(ctx, |model, _ctx| {
            mem::replace(&mut model.is_selected, selected)
        })
    }

    fn clone_boxed(&self) -> Box<dyn ChildModelHandle> {
        Box::new(self.clone())
    }
}
