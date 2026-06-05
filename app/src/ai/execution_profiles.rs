use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use warp_core::channel::ChannelState;
use warpui::{AppContext, SingletonEntity as _};

use crate::ai::llms::LLMId;
use crate::cloud_object::model::generic_string_model::{
    GenericStringModel, GenericStringObjectId, StringModel,
};
use crate::cloud_object::model::json_model::{JsonModel, JsonSerializer};
use crate::cloud_object::{
    GenericCloudObject, GenericStringObjectFormat, GenericStringObjectUniqueKey, JsonObjectType,
    UniquePer,
};
use crate::settings::{
    AISettings, AgentModeCommandExecutionPredicate, DEFAULT_COMMAND_EXECUTION_ALLOWLIST,
    DEFAULT_COMMAND_EXECUTION_DENYLIST,
};

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

impl RunAgentsPermission {
    pub fn description(&self) -> &'static str {
        match self {
            RunAgentsPermission::NeverAllow => "The Agent cannot run child agents and the run_agents tool will not be available.",
            RunAgentsPermission::AlwaysAllow => "Give the Agent full autonomy to run child agents without approval.",
            RunAgentsPermission::AlwaysAsk => "Require explicit approval before the Agent runs child agents.",
            RunAgentsPermission::Unknown => "Unknown setting.",
        }
    }
    pub fn is_enabled(&self) -> bool { matches!(self, Self::AlwaysAllow | Self::AlwaysAsk) }
    pub fn is_always_allow(&self) -> bool { matches!(self, Self::AlwaysAllow) }
    pub fn is_never_allow(&self) -> bool { matches!(self, Self::NeverAllow | Self::Unknown) }
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

impl AskUserQuestionPermission {
    pub fn label(&self) -> &'static str {
        match self {
            AskUserQuestionPermission::Never => "Never ask",
            AskUserQuestionPermission::AskExceptInAutoApprove => "Ask unless auto-approve",
            AskUserQuestionPermission::AlwaysAsk | AskUserQuestionPermission::Unknown => "Always ask",
        }
    }
    pub fn description(&self) -> &'static str {
        match self {
            AskUserQuestionPermission::AskExceptInAutoApprove | AskUserQuestionPermission::Unknown => "The Agent may ask a question and pause for your response, but will continue automatically when auto-approve is on.",
            AskUserQuestionPermission::Never => "The Agent will not ask questions and will continue with its best judgment.",
            AskUserQuestionPermission::AlwaysAsk => "The Agent may ask a question and will pause for your response even when auto-approve is on.",
        }
    }
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
    pub fn create_default_from_legacy_settings(app: &AppContext) -> Self {
        let ai_settings = AISettings::as_ref(app);
        Self {
            name: "Default".to_string(),
            is_default_profile: true,
            command_denylist: ai_settings.agent_mode_command_execution_denylist.clone(),
            command_allowlist: ai_settings
                .agent_mode_command_execution_allowlist
                .iter()
                .filter(|cmd| !DEFAULT_COMMAND_EXECUTION_ALLOWLIST.contains(cmd))
                .cloned()
                .collect(),
            directory_allowlist: ai_settings.agent_mode_coding_file_read_allowlist.clone(),
            ..Default::default()
        }
    }

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

    pub fn create_default_cli_profile(is_sandboxed: bool, computer_use_override: Option<bool>) -> Self {
        let command_denylist = if is_sandboxed { Vec::new() } else { DEFAULT_COMMAND_EXECUTION_DENYLIST.to_vec() };
        let computer_use_permission = match computer_use_override {
            Some(true) => {
                if is_sandboxed {
                    ComputerUsePermission::AlwaysAllow
                } else {
                    ComputerUsePermission::Never
                }
            }
            Some(false) => ComputerUsePermission::Never,
            None => {
                if is_sandboxed && ChannelState::channel().is_dogfood() {
                    ComputerUsePermission::AlwaysAllow
                } else {
                    ComputerUsePermission::Never
                }
            }
        };
        Self {
            name: "Default (CLI)".to_owned(),
            is_default_profile: true,
            apply_code_diffs: ActionPermission::AlwaysAllow,
            read_files: ActionPermission::AlwaysAllow,
            execute_commands: ActionPermission::AlwaysAllow,
            mcp_permissions: ActionPermission::AlwaysAllow,
            write_to_pty: WriteToPtyPermission::AlwaysAllow,
            ask_user_question: AskUserQuestionPermission::Never,
            run_agents: RunAgentsPermission::AlwaysAllow,
            command_denylist,
            command_allowlist: DEFAULT_COMMAND_EXECUTION_ALLOWLIST.to_vec(),
            directory_allowlist: Vec::new(),
            mcp_allowlist: Vec::new(),
            mcp_denylist: Vec::new(),
            computer_use: computer_use_permission,
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
    pub use super::{
        ActionPermission, AIExecutionProfile, AskUserQuestionPermission,
        ComputerUsePermission,
        RunAgentsPermission, WriteToPtyPermission,
    };
    use crate::ai::llms::LLMId;

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ClientProfileId(pub usize);
    impl std::fmt::Display for ClientProfileId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
    }

    pub struct AIExecutionProfileInfo {
        id: ClientProfileId,
        data: AIExecutionProfile,
    }
    impl AIExecutionProfileInfo {
        fn default_info() -> Self { Self { id: ClientProfileId(0), data: AIExecutionProfile::default() } }
        pub fn id(&self) -> &ClientProfileId { &self.id }
        pub fn data(&self) -> &AIExecutionProfile { &self.data }
        pub fn sync_id(&self) -> Option<crate::server::ids::SyncId> { None }
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
        pub fn default_profile(&self, _ctx: &warpui::AppContext) -> AIExecutionProfileInfo { AIExecutionProfileInfo::default_info() }
        pub fn default_profile_id(&self) -> ClientProfileId { ClientProfileId(0) }
        pub fn set_active_profile(&mut self, _view_id: warpui::EntityId, _id: ClientProfileId, _ctx: &mut ModelContext<Self>) {}
        pub fn create_profile(&mut self, _ctx: &mut ModelContext<Self>) -> Option<ClientProfileId> { None }
        pub fn delete_profile(&mut self, _id: ClientProfileId, _ctx: &mut ModelContext<Self>) {}
        pub fn get_profile_by_id(&self, _id: ClientProfileId, _ctx: &warpui::AppContext) -> Option<AIExecutionProfileInfo> { None }
        pub fn get_all_profile_ids(&self) -> Vec<ClientProfileId> { vec![] }
        pub fn get_profile_id_by_sync_id(&self, _sync_id: &crate::server::ids::SyncId) -> Option<ClientProfileId> { None }
        pub fn has_multiple_profiles(&self) -> bool { false }
        pub fn reset(&mut self) {}
        pub fn set_base_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_coding_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_cli_agent_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_computer_use_model(&mut self, _id: ClientProfileId, _model: Option<LLMId>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_context_window_limit(&mut self, _id: ClientProfileId, _limit: Option<u32>, _ctx: &mut ModelContext<Self>) {}
        pub fn set_apply_code_diffs(&mut self, _id: ClientProfileId, _p: &ActionPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_read_files(&mut self, _id: ClientProfileId, _p: &ActionPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_execute_commands(&mut self, _id: ClientProfileId, _p: &ActionPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_write_to_pty(&mut self, _id: ClientProfileId, _p: &WriteToPtyPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_mcp_permissions(&mut self, _id: ClientProfileId, _p: &ActionPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_computer_use(&mut self, _id: ClientProfileId, _p: &ComputerUsePermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_ask_user_question(&mut self, _id: ClientProfileId, _p: AskUserQuestionPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_run_agents(&mut self, _id: ClientProfileId, _p: RunAgentsPermission, _ctx: &mut ModelContext<Self>) {}
        pub fn set_web_search_enabled(&mut self, _id: ClientProfileId, _v: bool, _ctx: &mut ModelContext<Self>) {}
        pub fn set_autosync_plans_to_warp_drive(&mut self, _id: ClientProfileId, _v: bool, _ctx: &mut ModelContext<Self>) {}
        pub fn set_profile_name(&mut self, _id: ClientProfileId, _name: String, _ctx: &mut ModelContext<Self>) {}
        pub fn add_to_command_allowlist(&mut self, _id: ClientProfileId, _cmd: &crate::settings::AgentModeCommandExecutionPredicate, _ctx: &mut ModelContext<Self>) {}
        pub fn remove_from_command_allowlist(&mut self, _id: ClientProfileId, _idx: usize, _ctx: &mut ModelContext<Self>) {}
        pub fn add_to_command_denylist(&mut self, _id: ClientProfileId, _cmd: &crate::settings::AgentModeCommandExecutionPredicate, _ctx: &mut ModelContext<Self>) {}
        pub fn remove_from_command_denylist(&mut self, _id: ClientProfileId, _idx: usize, _ctx: &mut ModelContext<Self>) {}
        pub fn add_to_directory_allowlist(&mut self, _id: ClientProfileId, _path: &std::path::PathBuf, _ctx: &mut ModelContext<Self>) {}
        pub fn remove_from_directory_allowlist(&mut self, _id: ClientProfileId, _idx: usize, _ctx: &mut ModelContext<Self>) {}
        pub fn add_to_mcp_allowlist(&mut self, _id: ClientProfileId, _uuid: &uuid::Uuid, _ctx: &mut ModelContext<Self>) {}
        pub fn remove_from_mcp_allowlist(&mut self, _id: ClientProfileId, _uuid: &uuid::Uuid, _ctx: &mut ModelContext<Self>) {}
        pub fn add_to_mcp_denylist(&mut self, _id: ClientProfileId, _uuid: &uuid::Uuid, _ctx: &mut ModelContext<Self>) {}
        pub fn remove_from_mcp_denylist(&mut self, _id: ClientProfileId, _uuid: &uuid::Uuid, _ctx: &mut ModelContext<Self>) {}
        #[cfg(test)]
        pub fn apply_cli_profile_defaults_for_test(&mut self, _id: ClientProfileId, _sandboxed: bool, _ctx: &mut ModelContext<Self>) {}
        #[cfg(test)]
        pub fn default_profile_id_for_test(&self) -> ClientProfileId { ClientProfileId(0) }
    }
}


pub mod model_menu_items {
    use crate::ai::llms::LLMInfo;

    pub fn is_auto(_llm: &LLMInfo) -> bool { true }
}
