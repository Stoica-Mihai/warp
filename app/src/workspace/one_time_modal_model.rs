use warpui::{Entity, ModelContext, SingletonEntity, WindowId};

pub struct OneTimeModalModel {
    target_window_id: Option<WindowId>,
}

impl OneTimeModalModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self { target_window_id: None }
    }

    pub fn is_any_modal_open(&self) -> bool {
        false
    }

    pub fn update_target_window_id(&mut self, window_id: WindowId, _ctx: &mut ModelContext<Self>) {
        self.target_window_id = Some(window_id);
    }
}

impl Entity for OneTimeModalModel {
    type Event = ();
}

impl SingletonEntity for OneTimeModalModel {}
