use serde::{Deserialize, Serialize};
use session_sharing_protocol::common::SessionId as SharedSessionId;
use warp_core::interval_timer::TimingDataPoint;

use crate::ai::agent::{
    AIAgentActionId, AIAgentInput as FullAIAgentInput, PassiveSuggestionTrigger,
};
use crate::ai::agent_management::notifications::NotificationSourceAgent;
use crate::ai::blocklist::agent_view::AgentViewEntryOrigin;
use crate::cloud_object::model::generic_string_model::GenericStringObjectId;
use crate::cloud_object::{GenericStringObjectFormat, ObjectType, Space};
use crate::drive::CloudObjectTypeAndId;
use crate::notebooks::telemetry::NotebookTelemetryAction;
use crate::notebooks::{NotebookId, NotebookLocation};
use crate::search::command_search::searcher::CommandSearchItemAction;
use crate::server::ids::{ObjectUid, ServerId};
use crate::terminal::model::session::SessionId;
use crate::workflows::{WorkflowId, WorkflowSelectionSource, WorkflowSource};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum DownloadSource {
    Website,
    Homebrew,
}

// For use when recording what type of cloud object a particular telemetry is for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryCloudObjectType {
    Workflow,
    Notebook,
    Folder,
    GenericStringObject(GenericStringObjectFormat),
}

impl From<&CloudObjectTypeAndId> for TelemetryCloudObjectType {
    fn from(cloud_object_type_and_id: &CloudObjectTypeAndId) -> Self {
        match cloud_object_type_and_id {
            CloudObjectTypeAndId::Notebook(_) => Self::Notebook,
            CloudObjectTypeAndId::Workflow(_) => Self::Workflow,
            CloudObjectTypeAndId::Folder(_) => Self::Folder,
            CloudObjectTypeAndId::GenericStringObject { object_type, .. } => {
                Self::GenericStringObject(*object_type)
            }
        }
    }
}

/// For use when recording how a user has access to a cloud object.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TelemetrySpace {
    /// The object is owned by the current user.
    Personal,
    /// The object is owned by a team the user is on.
    Team,
    /// The object was shared with the user.
    Shared,
}

impl From<Space> for TelemetrySpace {
    fn from(space: Space) -> Self {
        match space {
            Space::Personal => Self::Personal,
            Space::Team { .. } => Self::Team,
            Space::Shared => Self::Shared,
        }
    }
}

/// Common metadata to include in all Warp Drive telemetry events that act on a specific object.
/// Events that only apply to a single object type may use specific metadata like [`WorkflowTelemetryMetadata`],
/// [`NotebookTelemetryMetadata`], or [`EnvVarTelemetryMetadata`] instead.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudObjectTelemetryMetadata {
    pub object_type: TelemetryCloudObjectType,
    /// The server UID of the object. This only exists for objects that have been synced to the
    /// server.
    pub object_uid: Option<ServerId>,
    /// The space through which the user has access to the object.
    pub space: Option<TelemetrySpace>,
    /// If the object is owned by a team, this is the owning team's UID. For shared objects, the
    /// user might not be on the team.
    pub team_uid: Option<ServerId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowTelemetryMetadata {
    pub workflow_categories: Option<Vec<String>>,
    pub workflow_source: WorkflowSource,
    pub workflow_space: Option<TelemetrySpace>,
    pub workflow_selection_source: WorkflowSelectionSource,
    // This field is only populated for cloud workflows that have been synced to the server
    pub workflow_id: Option<WorkflowId>,
    // Any referenced workflow enums that have been synced to the cloud
    pub enum_ids: Vec<GenericStringObjectId>,
}

/// Metadata to include in all notebook telemetry events.
///
/// There are 4 expected configurations:
/// * Personal cloud notebooks: `notebook_id` is `Some`, `team_uid` is `None`, and location is `PersonalCloud`
/// * Team cloud notebooks: `notebook_id` is `Some`, `team_uid` is `Some`, and location is `Team`
/// * Local file-based notebooks: `notebook_id` and `team_uid` are `None`, and location is `LocalFile`
/// * Remote file-based notebooks: `notebook_id` and `team_uid` are `None`, and location is `RemoteFile`
///
/// This representation allows for invalid combinations, but makes querying the data easier (for
/// example, to find all notebook events for a given team).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct NotebookTelemetryMetadata {
    /// The notebook ID, only available for cloud notebooks that have been synced to the server.
    pub notebook_id: Option<NotebookId>,
    /// The team UID, only available for cloud notebooks in a shared team.
    pub team_uid: Option<ServerId>,
    pub space: Option<TelemetrySpace>,
    /// Where the notebook is canonically located.
    pub location: NotebookLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_table_count: Option<usize>,
}

impl NotebookTelemetryMetadata {
    pub fn new(
        notebook_id: impl Into<Option<NotebookId>>,
        team_uid: impl Into<Option<ServerId>>,
        location: impl Into<NotebookLocation>,
        space: Option<TelemetrySpace>,
    ) -> Self {
        Self {
            notebook_id: notebook_id.into(),
            team_uid: team_uid.into(),
            location: location.into(),
            space,
            markdown_table_count: None,
        }
    }

    pub fn with_markdown_table_count(mut self, markdown_table_count: usize) -> Self {
        self.markdown_table_count = Some(markdown_table_count);
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnvVarTelemetryMetadata {
    /// The object ID, only available for cloud env vars that have been synced to the server.
    pub object_id: Option<GenericStringObjectId>,
    /// The team UID, only available for cloud env vars in a shared team.
    pub team_uid: Option<ServerId>,
    pub space: TelemetrySpace,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPTemplateInstallationSource {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "shared")]
    Shared,
    #[serde(rename = "gallery")]
    Gallery,
}

/// How the user opened the Warp Drive sharing dialog.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SharingDialogSource {
    /// The sharing button in the pane header.
    PaneHeader,
    /// The per-pane command palette entry (includes keybindings).
    CommandPalette,
    /// The Warp Drive index context menu.
    DriveIndex,
    /// The sharing dialog was auto-opened from shared session creation.
    StartedSessionShare,
    /// The user intented into Warp with an email address to invite.
    InviteeRequest,
    /// The user jumped from an inherited ACL to its definition on a parent object.
    InheritedPermission,
    /// The onboarding block shown after users create new personal objects.
    OnboardingBlock,
    /// The conversation list overflow menu.
    ConversationList,
    /// The AI block context menu.
    AIBlockContextMenu,
}

/// The possible sources notifications can turned on from.
/// The possible types of toggles in the find bar
#[derive(Clone, Serialize, Deserialize)]
pub enum FindOption {
    CaseSensitive,
    FindInBlock,
    Regex,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum LinkOpenMethod {
    CmdClick,
    ToolTip,
    MiddleClick,
}

/// The possible ways to trigger command x-ray
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandXRayTrigger {
    Hover,
    Keystroke,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum PaletteSource {
    PrefixChange,
    Keybinding,
    CtrlTab { shift_pressed_initially: bool },
    WarpDrive,
    QuitModal,
    LogOutModal,
    IntegrationTest,
    ConversationManager,
    ContextChip,
    PaneHeader,
    AgentTip,
    TitleBarSearchBar,
}

/// The CLI agent being used (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CLIAgentType {
    Claude,
    Gemini,
    Codex,
    Amp,
    Droid,
    OpenCode,
    Copilot,
    Pi,
    Auggie,
    Cursor,
    Goose,
    Hermes,
    Vibe,
    Unknown,
}

/// The kind of plugin chip shown or dismissed (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginChipTelemetryKind {
    Install,
    Update,
}

/// Identifies the agent variant that triggered a notification (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAgentVariant {
    /// Warp's built-in agent (Oz).
    Oz,
    /// A CLI agent (e.g., Claude Code, Gemini CLI, etc.).
    CLIAgent(CLIAgentType),
}

impl From<NotificationSourceAgent> for NotificationAgentVariant {
    fn from(agent: NotificationSourceAgent) -> Self {
        match agent {
            NotificationSourceAgent::Oz { .. } => Self::Oz,
            NotificationSourceAgent::CLI { agent, .. } => Self::CLIAgent(agent.into()),
        }
    }
}

/// The action taken on a plugin chip (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloseTarget {
    App,
    Window,
    Tab,
    Pane,
    EditorTab,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum PtySpawnMode {
    /// The pty was spawned using the terminal server.
    TerminalServer,
    /// We tried to spawn the pty using the terminal server, but something went
    /// wrong so we fell back to spawning it directly.
    FallbackToDirect,
    /// The terminal server is not in use, and we spawned the pty directly
    /// (in tests, for example).
    Direct,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OpenedWarpAISource {
    GlobalEntryButton,
    HelpWithBlock,
    HelpWithTextSelection,
    FromAICommandSearch,
    WarmWelcome,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SaveAsWorkflowModalSource {
    Block,
    Input,
    WarpAIWorkflowCard,
    WarpAIPanel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LaunchConfigUiLocation {
    CommandPalette,
    AppMenu,
    TabMenu,
    Uri,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AICommandSearchEntrypoint {
    ShortHandTrigger,
    Keybinding,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnonymousUserSignupEntrypoint {
    HitDriveObjectLimit,
    LoginGatedFeature,
    SignUpButton,
    RenotificationBlock,
    SignUpAIPrompt,
    NextCommandSuggestionsUpgradeBanner,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptChoice {
    PS1,
    Default,
    Custom { builtin_chips: Vec<String> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToggleBlockFilterSource {
    /// This includes the keybinding and the command palette items.
    Binding,
    ContextMenu,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypointSelectionType {
    /// User entered Agent Mode by taking action on a blocklist text selection.
    Text,

    /// User entered Agent Mode by taking action on a block selection.
    Block,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypoint {
    /// The stars icon button in the tab bar.
    #[serde(rename = "tab_bar")]
    TabBar,

    /// This corresponds to _both_ triggering from the command palette and via keybinding.
    ///
    /// Unfortunately due to the way the command palette automatically surfaces any editable
    /// keybinding as an action, we don't have enough information to discern if the binding was
    /// triggered by the palette or keyboard.
    #[serde(rename = "new_pane_binding")]
    NewPaneBinding,

    /// The stars button in the hoverable block "toolbelt".
    #[serde(rename = "block_toolbelt")]
    BlockToolbelt,

    /// The "Ask Agent Mode" option from AI command search.
    #[serde(rename = "ai_command_search")]
    AICommandSearch,

    /// Context menu item(s) that attach a blocklist selection as context to an Agent Mode query.
    #[serde(rename = "context_menu")]
    ContextMenu {
        selection_type: AgentModeEntrypointSelectionType,
    },

    /// The Agent Mode chip in the prompt.
    #[serde(rename = "prompt_chip")]
    PromptChip,

    /// The Agent Management popup, where you can see all the most recent tasks for each terminal
    /// pane across all windows/tabs/panes.
    #[serde(rename = "agent_management_popup")]
    AgentManagementPopup,

    /// User manually switched between terminal and AI input modes in UDI interface
    #[serde(rename = "udi_terminal_input_switcher")]
    UDITerminalInputSwitcher,

    /// The agent management view, where you can see both local interactive and ambient agent tasks
    #[serde(rename = "agent_management_view")]
    AgentManagementView,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InteractionSource {
    Button,
    Keybinding,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionViewType {
    TerminalView,
    AgentView,
}

/// The entrypoint from which the rewind dialog was opened.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AgentModeRewindEntrypoint {
    /// The rewind button in the AI block header.
    Button,
    /// The context menu item "Rewind to before here".
    ContextMenu,
    /// The /rewind slash command.
    SlashCommand,
}

/// Reasons why we fell back to a prompt suggestion from a suggested code diff.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionFallbackReason {
    /// Code file had too many lines, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_lines")]
    FileTooManyLines,
    /// Code file had too many bytes, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_bytes")]
    FileTooManyBytes,
    /// Missing file, when looking up filepaths in local file system.
    #[serde(rename = "missing_file")]
    MissingFile,
    /// Failed to retrieve file from local file system.
    #[serde(rename = "failed_to_retrieve_file")]
    FailedToRetrieveFile,
    /// In an SSH/remote session.
    #[serde(rename = "ssh_remote_session")]
    SSHRemoteSession,
    /// No read files permission.
    #[serde(rename = "no_read_files_permission")]
    NoReadFilesPermission,
    /// AI query timeout.
    #[serde(rename = "ai_query_timeout")]
    AIQueryTimeout,
    /// Failed to send AI request.
    #[serde(rename = "failed_to_send_ai_request")]
    FailedToSendAIRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuUsageStats {
    /// The number of logical CPUs on the system.
    pub num_cpus: usize,

    /// The maximum CPU usage over the measurement interval.
    ///
    /// This number is in the range [0, num_cpus].  The CPU utilization, as a
    /// percentage, can be determined via `max_usage / num_cpus * 100`.
    pub max_usage: f32,

    /// The average CPU usage over the measurement interval.
    ///
    /// This number is in the range [0, num_cpus].  The CPU utilization, as a
    /// percentage, can be determined via `avg_usage / num_cpus * 100`.
    pub avg_usage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub total_application_usage_bytes: usize,
    pub total_blocks: usize,
    pub total_lines: usize,

    /// Statistics about blocks that have been seen in the past 5 minutes.
    pub active_block_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [5m, 1h).
    pub inactive_5m_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [1h, 24h).
    pub inactive_1h_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [24h, ..).
    pub inactive_24h_stats: BlockMemoryUsageStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMemoryUsageStats {
    pub num_blocks: usize,
    pub num_lines: usize,
    pub estimated_memory_usage_bytes: usize,
}

/// Entrypoints to toggle the input auto-detection setting for Agent Mode.
/// Payload for the [`AgentModePotentialAutodetectionFalsePositive`] event.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentModeAutoDetectionFalsePositivePayload {
    /// Payload includes input text for dogfood channels.
    InternalDogfoodUsers { input_text: String },

    /// Do not include the misclassified input text in stable channels due to privacy concerns.
    ExternalUsers,
}

/// How the user triggered the [`AgentModeCodeFilesNavigated`] event.
/// How the user triggered the [`AddTabWithShell`] event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AddTabWithShellSource {
    CommandPalette,
    ShellSelectorMenu,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeContextDestination {
    Pty,
    AgentInput,
    RichInput,
}

#[derive(Clone, Debug, Serialize)]
pub enum AgentModeCitation {
    WarpDriveObject {
        object_type: ObjectType,
        uid: ObjectUid,
    },
    WarpDocs {
        page: String,
    },
    WebPage {
        // Don't serialize the URL to avoid leaking sensitive information.
        #[serde(skip_serializing)]
        url: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ImageProtocol {
    Kitty,
    ITerm,
}

#[derive(Clone, Debug, Serialize)]
pub enum AIAgentInput {
    UserQuery { query: String },
    AutoCodeDiffQuery { query: String },
    ResumeConversation,
    InitProjectRules { display_query: Option<String> },
    CreateEnvironment { display_query: Option<String> },
    TriggerSuggestPrompt { trigger: PassiveSuggestionTrigger },
    ActionResult { action_id: AIAgentActionId },
    CreateNewProject { query: String },
    CloneRepository { url: String },
    CodeReview,
    FetchReviewComments,
    SummarizeConversation,
    InvokeSkill { skill_name: String },
    StartFromAmbientRunPrompt,
    MessagesReceivedFromAgents { message_count: usize },
    EventsFromAgents { event_count: usize },
    PassiveSuggestionResult,
    OrchestrationConfigUpdate,
}

impl From<FullAIAgentInput> for AIAgentInput {
    fn from(input: FullAIAgentInput) -> Self {
        match input {
            FullAIAgentInput::UserQuery { query, .. } => Self::UserQuery { query },
            FullAIAgentInput::AutoCodeDiffQuery { query, .. } => Self::AutoCodeDiffQuery { query },
            FullAIAgentInput::ResumeConversation { .. } => Self::ResumeConversation,
            FullAIAgentInput::InitProjectRules { display_query, .. } => {
                Self::InitProjectRules { display_query }
            }
            FullAIAgentInput::CreateEnvironment { display_query, .. } => {
                Self::CreateEnvironment { display_query }
            }
            FullAIAgentInput::TriggerPassiveSuggestion { trigger, .. } => {
                Self::TriggerSuggestPrompt { trigger }
            }
            FullAIAgentInput::ActionResult { result, .. } => Self::ActionResult {
                action_id: result.id,
            },
            FullAIAgentInput::CreateNewProject { query, .. } => Self::CreateNewProject { query },
            FullAIAgentInput::CloneRepository { clone_repo_url, .. } => Self::CloneRepository {
                url: clone_repo_url.into_url(),
            },
            FullAIAgentInput::CodeReview { .. } => Self::CodeReview,
            FullAIAgentInput::FetchReviewComments { .. } => Self::FetchReviewComments,
            FullAIAgentInput::SummarizeConversation { .. } => Self::SummarizeConversation,
            FullAIAgentInput::InvokeSkill { skill, .. } => Self::InvokeSkill {
                skill_name: skill.name.clone(),
            },
            FullAIAgentInput::StartFromAmbientRunPrompt { .. } => Self::StartFromAmbientRunPrompt,
            FullAIAgentInput::MessagesReceivedFromAgents { messages } => {
                Self::MessagesReceivedFromAgents {
                    message_count: messages.len(),
                }
            }
            FullAIAgentInput::EventsFromAgents { events } => Self::EventsFromAgents {
                event_count: events.len(),
            },
            FullAIAgentInput::PassiveSuggestionResult { .. } => Self::PassiveSuggestionResult,
            FullAIAgentInput::OrchestrationConfigUpdate { .. } => Self::OrchestrationConfigUpdate,
        }
    }
}

/// The origin of an agent view entry, for telemetry purposes.
/// Details about which type of slash command was accepted
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteCodebaseIndexStatusTelemetrySource {
    Snapshot,
    PushUpdate,
    MutationResponse,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteCodebaseAutoIndexTrigger {
    NavigatedToGitRepo,
    CodebaseContextEnablementChanged,
}
