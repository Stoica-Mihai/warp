use warpui::elements::Empty;
use warpui::keymap::FixedBinding;
use warpui::{AppContext, Element, Entity, EntityId, TypedActionView, View, ViewContext};

pub struct AgentTodosPopupView {
    terminal_view_id: EntityId,
}

#[derive(Debug, Clone, Copy)]
pub enum AgentTodosPopupAction {
    ClosePopup,
}

pub enum AgentTodosPopupEvent {
    Close,
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        AgentTodosPopupAction::ClosePopup,
        id!(AgentTodosPopupView::ui_name()),
    )]);
}

impl AgentTodosPopupView {
    pub fn new(terminal_view_id: EntityId, _ctx: &mut ViewContext<Self>) -> Self {
        Self { terminal_view_id }
    }

    pub fn scroll_to_in_progress_item(&self) {}

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(AgentTodosPopupEvent::Close);
    }
}

impl View for AgentTodosPopupView {
    fn ui_name() -> &'static str {
        "AgentTodosPopup"
    }

    fn render(&self, _app: &warpui::AppContext) -> Box<dyn warpui::Element> {
        Empty::new().finish()
    }
}

impl Entity for AgentTodosPopupView {
    type Event = AgentTodosPopupEvent;
}

impl TypedActionView for AgentTodosPopupView {
    type Action = AgentTodosPopupAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AgentTodosPopupAction::ClosePopup => {
                self.close(ctx);
            }
        }
    }
}
