use std::sync::Arc;

use warp_core::ui::appearance::Appearance;
use warpui::elements::Empty;
use warpui::{AppContext, Element, Entity, TypedActionView, View, ViewContext};

use crate::ai::blocklist::BlocklistAIInputModel;
use crate::ai::execution_profiles::profiles::ClientProfileId;
use crate::ai::llms::LLMId;
use crate::settings_view::SettingsSection;
use crate::terminal::model::terminal_model::TerminalModel;
use crate::terminal::view::ambient_agent::AmbientAgentViewModel;
use warpui::ModelHandle;

pub fn calculate_scaled_font_size(appearance: &Appearance) -> f32 {
    appearance.monospace_font_size()
}

pub fn calculate_max_profile_name_width(_appearance: &Appearance) -> f32 {
    100.0
}

pub enum ProfileModelSelectorEvent {
    OpenSettings(SettingsSection),
    MenuVisibilityChanged { open: bool },
    ToggleInlineModelSelector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileModelSelectorAction {
    SelectProfile(ClientProfileId),
    SelectModel(LLMId),
    ToggleProfileMenu,
    ToggleModelMenu,
}
pub struct ProfileModelSelector;

impl Entity for ProfileModelSelector {
    type Event = ProfileModelSelectorEvent;
}

impl View for ProfileModelSelector {
    fn ui_name() -> &'static str { "ProfileModelSelector" }
    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl TypedActionView for ProfileModelSelector {
    type Action = ProfileModelSelectorAction;
}

impl ProfileModelSelector {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _menu_positioning_provider: Arc<dyn crate::terminal::input::MenuPositioningProvider>,
        _terminal_view_id: warpui::EntityId,
        _input_model: ModelHandle<BlocklistAIInputModel>,
        _ambient_agent_view_model: Option<ModelHandle<AmbientAgentViewModel>>,
        _terminal_model: Arc<parking_lot::FairMutex<TerminalModel>>,
        _ctx: &mut ViewContext<Self>,
    ) -> Self { Self }

    pub fn selected_model_id(&self) -> Option<LLMId> { None }
    pub fn is_open(&self) -> bool { false }
    pub fn set_profile_menu_visibility(&mut self, _open: bool, _ctx: &mut ViewContext<Self>) {}
    pub fn set_model_menu_visibility(&mut self, _open: bool, _ctx: &mut ViewContext<Self>) {}
    pub fn set_blurred(&mut self, _blurred: bool, _ctx: &mut ViewContext<Self>) {}
    pub fn set_render_compact(&mut self, _compact: bool, _ctx: &mut ViewContext<Self>) {}
    pub fn model_menu_item_position_id(&self, _llm_id: &LLMId) -> String { String::new() }
}
