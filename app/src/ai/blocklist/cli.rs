use warpui::elements::Empty;
use warpui::{AppContext, Element, Entity, View};

pub enum CLISubagentViewEvent {}

pub struct CLISubagentView;

impl CLISubagentView {
    pub fn clear_all_selections(&mut self, _app: &AppContext) {}
    pub fn selected_text(&self, _app: &AppContext) -> Option<String> { None }
}

impl Entity for CLISubagentView {
    type Event = CLISubagentViewEvent;
}

impl View for CLISubagentView {
    fn ui_name() -> &'static str {
        "CLISubagentView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}
