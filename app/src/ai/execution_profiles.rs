use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPermission {
    AgentDecides,
    AlwaysAllow,
    AlwaysAsk,
    #[serde(other)]
    Unknown,
}

impl ActionPermission {
    pub fn description(&self) -> &'static str {
        match self {
            ActionPermission::AgentDecides | ActionPermission::Unknown => "The Agent chooses the safest path: acting on its own when confident, and asking for approval when uncertain.",
            ActionPermission::AlwaysAllow => "Give the Agent full autonomy  — no manual approval ever required.",
            ActionPermission::AlwaysAsk => "Require explicit approval before the Agent takes any action.",
        }
    }
    pub fn is_always_ask(&self) -> bool { matches!(self, Self::AlwaysAsk) }
    pub fn is_always_allow(&self) -> bool { matches!(self, Self::AlwaysAllow) }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteToPtyPermission {
    #[serde(alias = "Never")]
    AlwaysAllow,
    #[default]
    AlwaysAsk,
    AskOnFirstWrite,
    #[serde(other)]
    Unknown,
}

impl WriteToPtyPermission {
    pub fn description(&self) -> &'static str {
        match self {
            WriteToPtyPermission::AlwaysAllow => ActionPermission::AlwaysAllow.description(),
            WriteToPtyPermission::AskOnFirstWrite => "The agent will ask for permission the first time it needs to interact with a running command. After that, it will continue automatically for the rest of that command.",
            WriteToPtyPermission::AlwaysAsk => "The agent will always ask for permission to interact with a running command.",
            WriteToPtyPermission::Unknown => ActionPermission::Unknown.description(),
        }
    }
    pub fn is_always_allow(&self) -> bool { matches!(self, Self::AlwaysAllow) }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputerUsePermission {
    #[default]
    Never,
    AlwaysAsk,
    AlwaysAllow,
    #[serde(other)]
    Unknown,
}

impl ComputerUsePermission {
    pub fn description(&self) -> &'static str {
        match self {
            ComputerUsePermission::Never => "Computer use tools are disabled and will not be available to the Agent.",
            ComputerUsePermission::AlwaysAsk => "Require explicit approval before the Agent uses computer use tools.",
            ComputerUsePermission::AlwaysAllow => "Give the Agent full autonomy to use computer use tools without approval.",
            ComputerUsePermission::Unknown => "Unknown setting.",
        }
    }
    pub fn is_enabled(&self) -> bool { !matches!(self, Self::Never | Self::Unknown) }
    pub fn is_always_allow(&self) -> bool { matches!(self, Self::AlwaysAllow) }
}

pub mod profiles {
    use warpui::{Entity, ModelContext, SingletonEntity};

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ClientProfileId(pub usize);
    impl std::fmt::Display for ClientProfileId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
    }

    #[allow(dead_code)]
    pub enum AIExecutionProfilesModelEvent {
        UpdatedActiveProfile { terminal_view_id: warpui::EntityId },
        ProfileUpdated(ClientProfileId),
        ProfilesChanged,
    }
    pub struct AIExecutionProfilesModel;
    impl Entity for AIExecutionProfilesModel {
        type Event = AIExecutionProfilesModelEvent;
    }
    impl SingletonEntity for AIExecutionProfilesModel {}
    impl AIExecutionProfilesModel {
        pub fn new(_launch_mode: &crate::LaunchMode, _ctx: &mut ModelContext<Self>) -> Self { Self }
    }
}
