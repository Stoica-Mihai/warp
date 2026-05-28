use warpui::{AppContext, Entity, SingletonEntity};

#[macro_export]
macro_rules! send_telemetry_from_ctx {
    ($event:expr, $ctx:expr) => {{}};
}

#[macro_export]
macro_rules! send_telemetry_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {{}};
}

pub trait TelemetryContextProvider {
    fn user_id(&self, ctx: &AppContext) -> Option<String>;

    fn anonymous_id(&self, ctx: &AppContext) -> String;
}

pub type TelemetryContextModel = Box<dyn TelemetryContextProvider>;

impl Entity for TelemetryContextModel {
    type Event = ();
}

impl SingletonEntity for TelemetryContextModel {}
