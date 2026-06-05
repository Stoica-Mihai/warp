use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use warpui::AppContext;

use crate::ai::llms::LLMId;
use crate::cloud_object::model::generic_string_model::{
    GenericStringModel, GenericStringObjectId, StringModel,
};
use crate::cloud_object::model::json_model::{JsonModel, JsonSerializer};
use crate::cloud_object::{
    GenericCloudObject, GenericStringObjectFormat, GenericStringObjectUniqueKey, JsonObjectType,
    UniquePer,
};
use crate::settings::{AgentModeCommandExecutionPredicate, DEFAULT_COMMAND_EXECUTION_DENYLIST};

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

pub struct CloudAgentComputerUseState {
    pub enabled: bool,
    pub is_forced_by_org: bool,
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

    pub fn resolve_cloud_agent_state(_ctx: &AppContext) -> CloudAgentComputerUseState {
        CloudAgentComputerUseState { enabled: false, is_forced_by_org: false }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunAgentsPermission {
    NeverAllow,
    AlwaysAllow,
    #[default]
    AlwaysAsk,
    #[serde(other)]
    Unknown,
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AskUserQuestionPermission {
    Never,
    AskExceptInAutoApprove,
    #[default]
    AlwaysAsk,
    #[serde(other)]
    Unknown,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AIExecutionProfile {
    pub name: String,
    pub is_default_profile: bool,
    pub apply_code_diffs: ActionPermission,
    pub read_files: ActionPermission,
    pub execute_commands: ActionPermission,
    pub write_to_pty: WriteToPtyPermission,
    pub mcp_permissions: ActionPermission,
    pub ask_user_question: AskUserQuestionPermission,
    pub run_agents: RunAgentsPermission,
    pub command_denylist: Vec<AgentModeCommandExecutionPredicate>,
    pub command_allowlist: Vec<AgentModeCommandExecutionPredicate>,
    pub directory_allowlist: Vec<PathBuf>,
    pub mcp_allowlist: Vec<uuid::Uuid>,
    pub mcp_denylist: Vec<uuid::Uuid>,
    pub computer_use: ComputerUsePermission,
    pub base_model: Option<LLMId>,
    pub coding_model: Option<LLMId>,
    pub cli_agent_model: Option<LLMId>,
    pub computer_use_model: Option<LLMId>,
    pub context_window_limit: Option<u32>,
    pub autosync_plans_to_warp_drive: bool,
    pub web_search_enabled: bool,
}

impl Default for AIExecutionProfile {
    fn default() -> Self {
        Self {
            name: Default::default(),
            is_default_profile: false,
            apply_code_diffs: ActionPermission::AgentDecides,
            read_files: ActionPermission::AgentDecides,
            execute_commands: ActionPermission::AlwaysAsk,
            write_to_pty: WriteToPtyPermission::AlwaysAsk,
            mcp_permissions: ActionPermission::AgentDecides,
            ask_user_question: AskUserQuestionPermission::AlwaysAsk,
            run_agents: RunAgentsPermission::AlwaysAsk,
            command_denylist: DEFAULT_COMMAND_EXECUTION_DENYLIST.clone(),
            command_allowlist: Vec::new(),
            directory_allowlist: Vec::new(),
            mcp_allowlist: Vec::new(),
            mcp_denylist: Vec::new(),
            computer_use: ComputerUsePermission::Never,
            base_model: None,
            coding_model: None,
            cli_agent_model: None,
            computer_use_model: None,
            context_window_limit: None,
            autosync_plans_to_warp_drive: true,
            web_search_enabled: true,
        }
    }
}

impl AIExecutionProfile {
    #[cfg(feature = "agent_mode_evals")]
    pub fn create_agent_mode_eval_profile() -> Self {
        Self {
            name: "Agent Mode Eval".to_string(),
            is_default_profile: false,
            apply_code_diffs: ActionPermission::AlwaysAllow,
            read_files: ActionPermission::AlwaysAllow,
            execute_commands: ActionPermission::AlwaysAllow,
            write_to_pty: WriteToPtyPermission::AlwaysAllow,
            mcp_permissions: ActionPermission::AlwaysAllow,
            ask_user_question: AskUserQuestionPermission::Never,
            run_agents: RunAgentsPermission::AlwaysAllow,
            command_denylist: Vec::new(),
            command_allowlist: Vec::new(),
            directory_allowlist: Vec::new(),
            mcp_allowlist: Vec::new(),
            mcp_denylist: Vec::new(),
            computer_use: ComputerUsePermission::Never,
            base_model: None,
            coding_model: None,
            cli_agent_model: None,
            computer_use_model: None,
            context_window_limit: None,
            autosync_plans_to_warp_drive: false,
            web_search_enabled: true,
        }
    }


}

pub type CloudAIExecutionProfile =
    GenericCloudObject<GenericStringObjectId, CloudAIExecutionProfileModel>;
pub type CloudAIExecutionProfileModel = GenericStringModel<AIExecutionProfile, JsonSerializer>;

impl StringModel for AIExecutionProfile {
    type CloudObjectType = CloudAIExecutionProfile;
    fn model_type_name(&self) -> &'static str { "AIExecutionProfile" }
    fn should_enforce_revisions() -> bool { true }
    fn model_format() -> GenericStringObjectFormat { GenericStringObjectFormat::Json(JsonObjectType::AIExecutionProfile) }
    fn should_show_activity_toasts() -> bool { false }
    fn warn_if_unsaved_at_quit() -> bool { true }
    fn display_name(&self) -> String {
        if self.is_default_profile { "Default".to_string() }
        else if self.name.trim().is_empty() { "Untitled".to_string() }
        else { self.name.clone() }
    }
    fn should_clear_on_unique_key_conflict(&self) -> bool { true }
    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        self.is_default_profile.then_some(GenericStringObjectUniqueKey {
            key: "default".to_string(),
            unique_per: UniquePer::User,
        })
    }
    fn renders_in_warp_drive(&self) -> bool { false }
}

impl JsonModel for AIExecutionProfile {
    fn json_object_type() -> JsonObjectType { JsonObjectType::AIExecutionProfile }
}

// Stub sub-modules to satisfy existing imports like
// `crate::ai::execution_profiles::profiles::AIExecutionProfilesModel`.

pub mod profiles {
    use warpui::{Entity, ModelContext, SingletonEntity};
    pub use super::AIExecutionProfile;
    use crate::ai::llms::LLMId;

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ClientProfileId(pub usize);
    impl std::fmt::Display for ClientProfileId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
    }

    pub struct AIExecutionProfileInfo {
        data: AIExecutionProfile,
    }
    impl AIExecutionProfileInfo {
        fn default_info() -> Self { Self { data: AIExecutionProfile::default() } }
        pub fn data(&self) -> &AIExecutionProfile { &self.data }
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
        pub fn active_profile(&self, _view_id: Option<warpui::EntityId>, _ctx: &warpui::AppContext) -> AIExecutionProfileInfo { AIExecutionProfileInfo::default_info() }
        pub fn get_profile_by_id(&self, _id: ClientProfileId, _ctx: &warpui::AppContext) -> Option<AIExecutionProfileInfo> { None }
        pub fn get_all_profile_ids(&self) -> Vec<ClientProfileId> { vec![] }
        pub fn set_base_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_coding_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_cli_agent_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_computer_use_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_context_window_limit(&mut self, _id: ClientProfileId, _limit: Option<u32>, _ctx: &mut ModelContext<Self>) {}
    }
}


pub mod model_menu_items {
    use crate::ai::llms::LLMInfo;

    pub fn is_auto(_llm: &LLMInfo) -> bool { true }
}
