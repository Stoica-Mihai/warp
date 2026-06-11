//! Settings for Blocklist AI.
//!
//! These settings are currently used to configure the underlying model/API used to power the AI
//! UX, as well as small UX configurations.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use settings::{
    define_settings_group, RespectUserSyncSetting, Setting, SupportedPlatforms, SyncToCloud,
};
use strum_macros::EnumIter;
use warp_core::execution_mode::AppExecutionMode;
use warp_core::features::FeatureFlag;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, UpdateModel};

use crate::auth::AuthStateProvider;
use crate::report_if_error;
use crate::workspaces::user_workspaces::UserWorkspaces;

pub enum FocusedTerminalInfoEvent {
    TerminalInfoUpdated,
}

/// Singleton model that is used to track the remote sessions in the terminal.
/// Useful for organizations that have restrictions on using AI in sessions in
/// remote sessions.
#[derive(Default, Clone, Debug)]
pub struct FocusedTerminalInfo {
    contains_any_remote_blocks: bool,
    contains_any_restored_remote_blocks: bool,
}

impl FocusedTerminalInfo {
    pub fn new(_: &mut ModelContext<Self>) -> Self {
        Self {
            contains_any_remote_blocks: false,
            contains_any_restored_remote_blocks: false,
        }
    }

    pub fn contains_any_remote_blocks(&self) -> bool {
        self.contains_any_remote_blocks
    }

    pub fn contains_any_restored_remote_blocks(&self) -> bool {
        self.contains_any_restored_remote_blocks
    }

    /// Updates both remote blocks and restored blocks status in a single atomic operation.
    /// Only emits a TerminalInfoUpdated event if either value changes.
    /// Returns true if the event was emitted.
    pub fn update(
        &mut self,
        contains_any_remote_blocks: bool,
        contains_any_restored_remote_blocks: bool,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let remote_changed = self.contains_any_remote_blocks != contains_any_remote_blocks;
        let restored_changed =
            self.contains_any_restored_remote_blocks != contains_any_restored_remote_blocks;

        if remote_changed || restored_changed {
            self.contains_any_remote_blocks = contains_any_remote_blocks;
            self.contains_any_restored_remote_blocks = contains_any_restored_remote_blocks;
            ctx.emit(FocusedTerminalInfoEvent::TerminalInfoUpdated);
            return true;
        }

        false
    }
}

impl Entity for FocusedTerminalInfo {
    type Event = FocusedTerminalInfoEvent;
}

impl SingletonEntity for FocusedTerminalInfo {}

/// The default mode for new terminal sessions.
#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    EnumIter,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Default mode for new sessions.",
    rename_all = "snake_case"
)]
pub enum DefaultSessionMode {
    /// New sessions start in the terminal mode (default).
    #[default]
    Terminal,
    /// New sessions start in agent view.
    Agent,
    /// New sessions start in cloud (ambient) agent mode.
    CloudAgent,
    /// New sessions open a user-defined tab config.
    /// The specific config is identified by the companion `default_tab_config_path` setting.
    TabConfig,
}

settings::macros::implement_setting_for_enum!(
    DefaultSessionMode,
    AISettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "general.default_session_mode",
    description: "The default mode for new terminal sessions.",
);

impl DefaultSessionMode {
    /// Display name for the settings dropdown.
    pub fn display_name(&self) -> &'static str {
        match self {
            DefaultSessionMode::Terminal => "Terminal",
            DefaultSessionMode::Agent => "Agent",
            DefaultSessionMode::CloudAgent => "Cloud Oz",
            DefaultSessionMode::TabConfig => "Tab Config",
        }
    }
}

/// Controls how agent thinking/reasoning traces are displayed after streaming.
#[derive(
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Copy,
    Clone,
    EnumIter,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Controls how agent thinking is displayed after streaming.",
    rename_all = "snake_case"
)]
pub enum ThinkingDisplayMode {
    /// Show reasoning blocks while streaming, then collapse them when complete (default).
    #[default]
    ShowAndCollapse,
    /// Always keep reasoning blocks expanded, even after streaming finishes.
    AlwaysShow,
    /// Never show reasoning blocks.
    NeverShow,
}

settings::macros::implement_setting_for_enum!(
    ThinkingDisplayMode,
    AISettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
    private: false,
    toml_path: "agents.warp_agent.other.thinking_display_mode",
    description: "Controls how agent thinking traces are displayed after streaming.",
);

impl ThinkingDisplayMode {
    /// Display name for the settings dropdown.
    pub fn display_name(&self) -> &'static str {
        match self {
            ThinkingDisplayMode::ShowAndCollapse => "Show & collapse",
            ThinkingDisplayMode::AlwaysShow => "Always show",
            ThinkingDisplayMode::NeverShow => "Never show",
        }
    }

    pub fn command_palette_description(&self) -> &'static str {
        match self {
            ThinkingDisplayMode::ShowAndCollapse => "Set agent thinking display: show & collapse",
            ThinkingDisplayMode::AlwaysShow => "Set agent thinking display: always show",
            ThinkingDisplayMode::NeverShow => "Set agent thinking display: never show",
        }
    }

    pub fn should_render(&self) -> bool {
        !matches!(self, ThinkingDisplayMode::NeverShow)
    }

    pub fn should_keep_expanded(&self) -> bool {
        matches!(self, ThinkingDisplayMode::AlwaysShow)
    }
}

/// Tracks the state of the quota reset banner
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Default,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(description = "State of the quota reset banner.")]
pub struct BannerState {
    #[serde(default)]
    #[schemars(description = "Whether the banner has been dismissed.")]
    pub dismissed: bool,
}

/// Tracks information about a single billing cycle for AI request usage
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(description = "Information about a single billing cycle.")]
pub struct CycleInfo {
    /// End date of the billing cycle
    #[schemars(description = "End date of the billing cycle.")]
    pub end_date: DateTime<Utc>,
    /// Whether the quota was exceeded in this cycle
    #[schemars(description = "Whether the usage quota was exceeded in this cycle.")]
    pub was_quota_exceeded: bool,
    /// State of the quota reset banner
    #[schemars(description = "State of the quota reset banner for this cycle.")]
    pub banner_state: BannerState,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Default,
    PartialEq,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(description = "AI usage quota information across billing cycles.")]
pub struct AIRequestQuotaInfo {
    /// History of billing cycles and their usage.
    ///
    /// Note that these are only populated going forward from when this setting
    /// was introduced.
    #[schemars(description = "History of billing cycles and their quota usage.")]
    pub cycle_history: Vec<CycleInfo>,
}

/// Predicate types to match commands that can be executed by Agent Mode.
#[derive(Debug, Serialize, Deserialize, Clone)]
enum AgentModeCommandExecutionPredicateType {
    /// A regex with start (`^`) and end (`$`) anchors.
    ///
    /// We want regex rules to apply to the entire cmd string so we anchor them
    /// (there isn't any efficient way to apply to the entire cmd string at match-time).
    #[serde(with = "serde_regex")]
    AnchoredRegex(Regex),
}

impl AgentModeCommandExecutionPredicateType {
    fn new_regex(regex: &str) -> Result<Self, regex::Error> {
        // Redundant anchors aren't a problem so we can unconditionally add them.
        let anchored_regex = Regex::new(&format!("^{regex}$"))?;
        Ok(Self::AnchoredRegex(anchored_regex))
    }

    fn matches(&self, cmd: &str) -> bool {
        match self {
            Self::AnchoredRegex(regex) => regex.is_match(cmd),
        }
    }
}

impl PartialEq for AgentModeCommandExecutionPredicateType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AnchoredRegex(a), Self::AnchoredRegex(b)) => {
                // Indexing should be safe since they're guaranteed to have at least
                // the anchors around them.
                let a_unanchored = &a.as_str()[1..a.as_str().len() - 1];
                let b_unanchored = &b.as_str()[1..b.as_str().len() - 1];
                a_unanchored == b_unanchored
            }
        }
    }
}

impl std::fmt::Display for AgentModeCommandExecutionPredicateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnchoredRegex(regex) => {
                write!(f, "{}", &regex.as_str()[1..regex.as_str().len() - 1])
            }
        }
    }
}

/// A wrapper around [`AgentModeCommandExecutionPredicateType`] to enforce
/// the use of the provided constructors rather than direct construction of the variants.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct AgentModeCommandExecutionPredicate(AgentModeCommandExecutionPredicateType);

impl schemars::JsonSchema for AgentModeCommandExecutionPredicate {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("AgentModeCommandExecutionPredicate")
    }

    fn json_schema(gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // In the settings file, predicates are serialized as plain regex strings.
        gen.subschema_for::<String>()
    }
}

impl AgentModeCommandExecutionPredicate {
    pub fn new_regex(regex: &str) -> Result<Self, regex::Error> {
        Ok(Self(AgentModeCommandExecutionPredicateType::new_regex(
            regex,
        )?))
    }

    pub fn matches(&self, cmd: &str) -> bool {
        self.0.matches(cmd)
    }
}

impl std::fmt::Display for AgentModeCommandExecutionPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl settings_value::SettingsValue for AgentModeCommandExecutionPredicate {
    fn to_file_value(&self) -> serde_json::Value {
        serde_json::Value::String(self.to_string())
    }

    fn from_file_value(value: &serde_json::Value) -> Option<Self> {
        value.as_str().and_then(|s| Self::new_regex(s).ok())
    }
}


define_settings_group!(AISettings, settings: [
    // If `false`, all AI features are disabled.
    is_any_ai_enabled: IsAnyAIEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::No),
        private: false,
        toml_path: "agents.warp_agent.is_any_ai_enabled",
        description: "Controls whether all AI features are enabled.",
    },
    // This field should not be referenced directly to lookup active AI enablement -- use the
    // `is_active_ai_enabled()` getter.
    is_active_ai_enabled_internal: IsActiveAIEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::No),
        private: false,
        toml_path: "agents.warp_agent.active_ai.enabled",
        description: "Controls whether proactive AI features like suggestions are enabled.",
    },
    // This field should not be referenced directly to lookup autodetection enablement -- use the
    // `is_ai_autodetection_enabled()` getter.
    ai_autodetection_enabled_internal: AIAutoDetectionEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.input.ai_auto_detection_enabled",
        description: "Controls whether AI automatically detects natural language input.",
    },
    // This field should not be referenced directly -- use the
    // `is_nld_in_terminal_enabled()` getter.
    // Controls whether natural language detection is enabled in the terminal input.
    //
    // This is only used when `FeatureFlag::AgentView` is enabled.
    nld_in_terminal_enabled_internal: NLDInTerminalEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.input.nld_in_terminal_enabled",
        description: "Controls whether natural language detection is enabled in the terminal input.",
    },
    // This field should not be referenced directly to lookup intelligent autosuggestion enablement
    // -- use the `is_intelligent_autosuggestions_enabled()` getter.
    intelligent_autosuggestions_enabled_internal: IntelligentAutosuggestionsEnabled {
        type: bool,
        default: true, // TODO(roland): revisit this when launched to stable
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.active_ai.intelligent_autosuggestions_enabled",
        description: "Controls whether AI-powered intelligent autosuggestions are enabled.",
    }
    // This field should not be referenced directly to lookup Prompt Suggestions
    // enablement -- use the `is_prompt_suggestions_enabled()` getter.
    // Note that AgentModeQuerySuggestionsEnabled is a legacy name (the feature was initially named Agent
    // Mode Query Suggestions), however, we do not want to change the name of the setting key to avoid
    // breaking existing user settings.
    prompt_suggestions_enabled_internal: AgentModeQuerySuggestionsEnabled {
        type: bool,
        default: true, // TODO(advait): revisit this when launched to stable
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.active_ai.agent_mode_query_suggestions_enabled",
        description: "Controls whether prompt suggestions are shown in agent mode.",
    }

    // This field should not be referenced directly to lookup Code Suggestions
    // enablement -- use the `is_code_suggestions_enabled()` getter.
    code_suggestions_enabled_internal: CodeSuggestionsEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.active_ai.code_suggestions_enabled",
        description: "Controls whether AI code suggestions are enabled.",
    }
    // This field should not be referenced directly to lookup natural language autosuggestions
    // enablement -- use the `is_natural_language_autosuggestions_enabled()` getter.
    // This feature refers to ghosted text for AI input queries.
    natural_language_autosuggestions_enabled_internal: NaturalLanguageAutosuggestionsEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.active_ai.natural_language_autosuggestions_enabled",
        description: "Controls whether ghosted text autosuggestions are shown for AI input queries.",
    }
    // This field should not be referenced directly to lookup shared block title generations
    // enablement -- use the `is_shared_block_title_generation_enabled()` getter.
    // This feature refers to the auto title generation when the user opens the shared block dialog.
    shared_block_title_generation_enabled_internal: SharedBlockTitleGenerationEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.active_ai.shared_block_title_generation_enabled",
        description: "Controls whether titles are auto-generated when sharing blocks.",
    }
    // Information about AI request quotas and usage across billing cycles
    ai_request_quota_info: AIRequestQuotaInfoSetting {
        type: AIRequestQuotaInfo,
        default: AIRequestQuotaInfo::default(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: true,
    },

    mcp_execution_path: MCPExecutionPath {
        type: Option<String>,
        default: None,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
    },

    // Used to determine whether the "What's new in Oz" section of the agent view
    // zero state is shown or hidden.
    should_show_oz_updates_in_zero_state: ShouldShowOzUpdatesInZeroState {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.other.should_show_oz_updates_in_zero_state",
        description: "Whether the \"What's new\" section is shown in the agent view.",
    }
    // Controls whether Warp's built-in feedback skill is available to the Warp Agent.
    feedback_bundled_skill_enabled: FeedbackBundledSkillEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.other.feedback_bundled_skill_enabled",
        description: "Whether Warp's built-in feedback skill is available to the Warp Agent.",
    }


    should_render_use_agent_footer_for_user_commands: ShouldRenderUseAgentToolbarForUserCommands {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.other.should_render_use_agent_toolbar_for_user_commands",
        description: "Whether to show the \"Use Agent\" footer for terminal commands.",
    }

    // Whether to render the CLI agent footer for commands like Claude, Codex, Gemini, etc.
    // This is independent of the "Use Agent" footer setting.
    should_render_cli_agent_footer: ShouldRenderCLIAgentToolbar {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.third_party.should_render_cli_agent_toolbar",
        description: "Whether to show the CLI agent footer for coding agent commands.",
    }
    // When enabled and a CLI agent session has a plugin listener, rich input
    // auto-closes when the session enters a Blocked state (the agent requires
    // direct keyboard interaction) and auto-opens when it leaves Blocked.
    auto_toggle_rich_input: AutoToggleRichInput {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.third_party.auto_toggle_composer",
        description: "Whether CLI agent Rich Input automatically closes and reopens based on the agent's blocked state.",
    }

    // When enabled and a CLI agent session has a plugin listener, rich input
    // auto-opens once when the session starts or when the listener is registered.
    auto_open_rich_input_on_cli_agent_start: AutoOpenRichInputOnCLIAgentStart {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.third_party.auto_open_composer_on_cli_agent_start",
        description: "Whether CLI agent Rich Input automatically opens when a CLI agent session starts.",
    }




    // The raw stored default mode for new sessions. Use `default_session_mode()` to retrieve the
    // effective value, which is gated on AI availability.
    default_session_mode_internal: DefaultSessionMode,

    // The file path of the tab config used when default_session_mode_internal is TabConfig.
    // Only read when mode is TabConfig; ignored for all other modes.
    // Machine-local (tab config paths vary per machine), so never synced to cloud.
    default_tab_config_path: DefaultTabConfigPath {
        type: String,
        default: String::new(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "general.default_tab_config_path",
    }



    // Whether file-based MCP servers from third-party AI tools (e.g. Claude, Codex) should
    // be automatically detected and spawned. Warp-native config files (.warp/.mcp.json) are
    // always detected and spawned, regardless of this setting.
    file_based_mcp_enabled: FileBasedMcpEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.mcp_servers.file_based_mcp_enabled",
        description: "Whether third-party file-based MCP servers are automatically detected.",
    }

    // Controls how agent thinking/reasoning traces are displayed.
    thinking_display_mode: ThinkingDisplayMode,

    // Whether agent-executed shell commands should be included in command history
    // (up-arrow, Ctrl-R search, inline history menu).
    // When false, commands run by the AI agent are excluded from history.
    include_agent_commands_in_history: IncludeAgentCommandsInHistory {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.input.include_agent_commands_in_history",
        description: "Whether agent-executed commands are included in command history.",
    }

    // Controls whether agent notifications (mailbox button, toasts, notification items) are shown.
    show_agent_notifications: ShowAgentNotifications {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "agents.warp_agent.other.show_agent_notifications",
        description: "Whether agent notifications are shown.",
    }



]);

impl AISettings {
    pub fn register_and_subscribe_to_events(app: &mut AppContext) {
        Self::register(app);
        app.add_singleton_model(FocusedTerminalInfo::new);

        app.update_model(&Self::handle(app), |_me, ctx| {
            ctx.subscribe_to_model(&FocusedTerminalInfo::handle(ctx), |_me, event, ctx| {
                if matches!(event, FocusedTerminalInfoEvent::TerminalInfoUpdated) {
                    // Pipe the event so that any view that listens for settings changes will be notified.
                    ctx.emit(AISettingsChangedEvent::IsAnyAIEnabled {
                        change_event_reason: ChangeEventReason::LocalChange,
                    });
                }
            });
        });
    }

    pub fn is_ai_disabled_due_to_remote_session_org_policy(&self, app: &AppContext) -> bool {
        let contains_remote_blocks = FocusedTerminalInfo::as_ref(app).contains_any_remote_blocks();

        let contains_restored_remote_blocks =
            FocusedTerminalInfo::as_ref(app).contains_any_restored_remote_blocks();

        let is_ai_allowed_in_remote_sessions =
            UserWorkspaces::as_ref(app).is_ai_allowed_in_remote_sessions();

        if is_ai_allowed_in_remote_sessions {
            return false;
        }

        contains_remote_blocks || contains_restored_remote_blocks
    }

    pub fn is_any_ai_enabled(&self, app: &AppContext) -> bool {
        // Disable AI for anonymous and logged-out users.
        let is_anonymous_or_logged_out = AuthStateProvider::as_ref(app)
            .get()
            .is_anonymous_or_logged_out();

        *self.is_any_ai_enabled
            && !is_anonymous_or_logged_out
            && !self.is_ai_disabled_due_to_remote_session_org_policy(app)
    }

    pub fn default_session_mode(&self, app: &AppContext) -> DefaultSessionMode {
        let mode = *self.default_session_mode_internal.value();
        match mode {
            // Terminal and TabConfig don't require AI.
            DefaultSessionMode::Terminal | DefaultSessionMode::TabConfig => mode,
            // Agent and CloudAgent require AI to be enabled.
            DefaultSessionMode::Agent | DefaultSessionMode::CloudAgent => {
                if self.is_any_ai_enabled(app) {
                    mode
                } else {
                    DefaultSessionMode::Terminal
                }
            }
        }
    }

    /// Returns the stored default tab config path (only meaningful when mode is `TabConfig`).
    pub fn default_tab_config_path(&self) -> &str {
        &self.default_tab_config_path
    }

    /// Looks up the `TabConfig` matching the stored `default_tab_config_path`.
    /// Returns `None` if the path is empty or no loaded config matches.
    pub fn resolved_default_tab_config(
        &self,
        app: &AppContext,
    ) -> Option<crate::tab_configs::TabConfig> {
        let path_str = self.default_tab_config_path.as_str();
        if path_str.is_empty() {
            return None;
        }
        let path = std::path::Path::new(path_str);
        crate::user_config::WarpConfig::as_ref(app)
            .tab_configs()
            .iter()
            .find(|config| config.source_path.as_deref().is_some_and(|p| p == path))
            .cloned()
    }

    pub fn is_active_ai_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_any_ai_enabled(app)
            && *self.is_active_ai_enabled_internal
            && AppExecutionMode::as_ref(app).allows_active_ai()
    }

    pub fn is_prompt_suggestions_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_active_ai_enabled(app) && *self.prompt_suggestions_enabled_internal
    }

    pub fn is_code_suggestions_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_active_ai_enabled(app) && *self.code_suggestions_enabled_internal
    }

    pub fn is_natural_language_autosuggestions_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_active_ai_enabled(app) && *self.natural_language_autosuggestions_enabled_internal
    }

    pub fn is_shared_block_title_generation_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_active_ai_enabled(app) && *self.shared_block_title_generation_enabled_internal
    }

    pub fn is_intelligent_autosuggestions_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_active_ai_enabled(app) && *self.intelligent_autosuggestions_enabled_internal
    }

    /// Returns `true` if input autodetection is enabled.
    ///
    /// If `FeatureFlag::AgentView` is enabled, this specifically gates NLD enablement in the agent
    /// view only.
    pub fn is_ai_autodetection_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_any_ai_enabled(app) && *self.ai_autodetection_enabled_internal
    }

    /// Returns `true` if NLD is enabled in the terminal.
    ///
    /// This is only used when `FeatureFlag::AgentView` is enabled.
    /// If the user has not explicitly set this setting, it defaults to the value of
    /// `ai_autodetection_enabled_internal`.
    pub fn is_nld_in_terminal_enabled(&self, app: &warpui::AppContext) -> bool {
        self.is_any_ai_enabled(app) && *self.nld_in_terminal_enabled_internal
    }

    pub fn is_file_based_mcp_enabled(&self, app: &warpui::AppContext) -> bool {
        if !FeatureFlag::FileBasedMcp.is_enabled() || !self.is_any_ai_enabled(app) {
            return false;
        }
        // NOTE: we intentionally do not force-enable this in Cloud Mode. Previously
        // we auto-spawned file-based MCPs in autonomous execution, but that bypassed
        // the user's explicit opt-in and let any MCP config checked into a repo run
        // arbitrary commands as part of a cloud agent run. Respecting the toggle
        // closes that attack surface; cloud agents that need project-scoped MCP
        // servers should surface an explicit, auditable opt-in. A more robust
        // solution (e.g. per-environment allowlisting, signed configs) should be
        // explored in the future.
        *self.file_based_mcp_enabled
    }



    /// Determines whether a quota reset banner should be displayed to the user.
    ///
    /// Marks the banner as dismissed for all completed cycles.
    pub fn mark_quota_banner_as_dismissed(&mut self, ctx: &mut ModelContext<Self>) {
        let mut cycle_history = self.ai_request_quota_info.cycle_history.clone();

        for cycle in cycle_history.iter_mut() {
            if cycle.end_date < Utc::now() {
                cycle.banner_state.dismissed = true;
            }
        }

        report_if_error!(self
            .ai_request_quota_info
            .set_value(AIRequestQuotaInfo { cycle_history }, ctx));
    }

}


#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
