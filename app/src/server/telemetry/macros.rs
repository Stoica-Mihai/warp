//! Telemetry send macros — neutralized in this fork (telemetry removed).
//!
//! Each expands to an empty block so the existing call-sites stay valid while
//! sending nothing. The `$event`/`$ctx` args are intentionally not expanded, so
//! `TelemetryEvent` and its construction sites can be deleted without editing
//! every call-site first.

#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr, $ctx:expr) => {{}};
}

#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {{}};
}

#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state:expr, $event:expr, $executor:expr) => {{}};
}
