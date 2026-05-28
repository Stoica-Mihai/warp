#[macro_export]
macro_rules! send_telemetry_from_ctx {
    ($event:expr, $ctx:expr) => {{}};
}

#[macro_export]
macro_rules! send_telemetry_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {{}};
}
