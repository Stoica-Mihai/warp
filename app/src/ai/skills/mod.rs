mod telemetry;
pub use telemetry::SkillOpenOrigin;

cfg_if::cfg_if! {
    if #[cfg(not(feature = "local_fs"))] {
        mod dummy_skill_manager;
        pub use dummy_skill_manager::SkillManager;
    }
}



mod listed_skill;
pub use listed_skill::SkillDescriptor;

mod skill_utils;


cfg_if::cfg_if! {
    if #[cfg(feature = "local_fs")] {
        mod skill_manager;
        pub use skill_manager::SkillManager;
        
    }
}
