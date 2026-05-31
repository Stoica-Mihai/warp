//! UI state for the ambient agent progress/loading screen.

use warpui::elements::shimmering_text::ShimmeringTextStateHandle;
use warpui::elements::{MouseStateHandle, SelectionHandle};
use crate::terminal::view::ambient_agent::model::AmbientAgentViewModel;

/// UI state for rendering the ambient agent progress screen (loading or error).
pub struct AmbientAgentProgressUIState {
    pub loading_shimmer_handle: ShimmeringTextStateHandle,
    pub error_selection_handle: SelectionHandle,
    pub error_selected_text: std::rc::Rc<parking_lot::RwLock<Option<String>>>,
    pub auth_button_mouse_state: MouseStateHandle,
}

impl AmbientAgentProgressUIState {
    pub fn new(_ctx: &mut warpui::ModelContext<AmbientAgentViewModel>) -> Self {
        Self {
            loading_shimmer_handle: ShimmeringTextStateHandle::new(),
            error_selection_handle: SelectionHandle::default(),
            error_selected_text: std::rc::Rc::new(parking_lot::RwLock::new(None)),
            auth_button_mouse_state: MouseStateHandle::default(),
        }
    }
}
