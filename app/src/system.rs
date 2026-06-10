cfg_if::cfg_if! {
    if #[cfg(not(target_family = "wasm"))] {
        mod info;
        mod memory_footprint;
        pub use info::SystemInfo;
    }
}

use warpui::SingletonEntity;

pub fn long_os_version(ctx: &warpui::AppContext) -> Option<String> {
    crate::system::SystemInfo::as_ref(ctx)
        .long_os_version()
        .map(ToOwned::to_owned)
}

#[cfg(target_family = "wasm")]
pub fn long_os_version(_ctx: &warpui::AppContext) -> Option<String> {
    None
}
