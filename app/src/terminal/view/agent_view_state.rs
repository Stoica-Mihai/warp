//! Terminal-pane view state describing whether the pane is showing an agent view, plus the
//! small rendering helpers that depend on that state. Relocated here from the removed
//! `ai/blocklist/agent_view` module since this state is load-bearing for terminal block
//! visibility and layout independent of the cloud agent UI.
use std::time::Duration;

use pathfinder_color::ColorU;
use warp_core::ui::appearance::Appearance;
use warpui::elements::Container;
use warpui::prelude::{Border, CornerRadius, Radius};
use warpui::{AppContext, Element, SingletonEntity};

use crate::ai::conversation_types::AIConversationId;
use crate::terminal::input::slash_commands::SlashCommandTrigger;

/// The display mode for an active agent view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentViewDisplayMode {
    /// Full-screen agent view (navstack-based).
    FullScreen,
    /// Inline agent view (e.g., for long-running commands).
    Inline,
}

impl AgentViewDisplayMode {
    pub fn is_inline(self) -> bool {
        matches!(self, AgentViewDisplayMode::Inline)
    }

    pub fn is_fullscreen(self) -> bool {
        matches!(self, AgentViewDisplayMode::FullScreen)
    }
}

/// Shared timeout for all "press again to confirm" UX in and around agent view.
pub const ENTER_OR_EXIT_CONFIRMATION_WINDOW: Duration = Duration::from_secs(1);

/// The different types of agent view entrypoints.
///
/// Depending on the entrypoint, an `AgentView` block representing the entry may be inserted into
/// the terminal blocklist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentViewEntryOrigin {
    /// Entered agent view from user input (e.g. /agent or cmd-enter keypress).
    Input {
        was_prompt_autodetected: bool,
    },
    PromptChip,
    ConversationSelector,
    AgentModeHomepage,
    AgentViewBlock,
    AIDocument,
    AutoFollowUp,
    RestoreExistingConversation,
    SharedSessionSelection,
    AgentRequestedNewConversation,
    AcceptedPromptSuggestion,
    AcceptedUnitTestSuggestion,
    AcceptedPassiveCodeDiff,
    InlineCodeReview,
    CloudAgent,
    ThirdPartyCloudAgent,
    Cli,
    ImageAdded,
    SlashCommand {
        trigger: SlashCommandTrigger,
    },
    SlashInit,
    CreateEnvironment,
    Keybinding,
    CodeReviewContext,
    InlineHistoryMenu,
    InlineConversationMenu,
    OnboardingCallout,
    DefaultSessionMode,
    LongRunningCommand,
    Onboarding,
    ChildAgent,
    OrchestrationPillBar,
    ProjectEntry,
    LinearDeepLink,
    ViewPassiveCodeDiffDetails,
    ResumeConversationButton,
    ContinueConversationButton,
    ClearBuffer,
}

impl AgentViewEntryOrigin {
    pub fn is_cloud_agent(&self) -> bool {
        matches!(self, Self::CloudAgent)
    }
}

/// Terminal view-scoped state representing whether or not the user is engaged in an active agent view.
#[derive(Debug, Clone)]
pub enum AgentViewState {
    Active {
        conversation_id: AIConversationId,
        origin: AgentViewEntryOrigin,
        display_mode: AgentViewDisplayMode,
        original_conversation_length: usize,
    },
    Inactive,
}

impl AgentViewState {
    pub fn active_conversation_id(&self) -> Option<AIConversationId> {
        match self {
            AgentViewState::Active {
                conversation_id, ..
            } => Some(*conversation_id),
            AgentViewState::Inactive => None,
        }
    }

    /// Returns the display mode if active, `None` if inactive.
    pub fn display_mode(&self) -> Option<AgentViewDisplayMode> {
        match self {
            AgentViewState::Active { display_mode, .. } => Some(*display_mode),
            AgentViewState::Inactive => None,
        }
    }

    pub fn origin(&self) -> Option<AgentViewEntryOrigin> {
        match self {
            AgentViewState::Active { origin, .. } => Some(*origin),
            AgentViewState::Inactive => None,
        }
    }

    /// Returns `true` if in an active agent view state.
    pub fn is_active(&self) -> bool {
        matches!(self, AgentViewState::Active { .. })
    }

    /// Returns `true` if in inline display mode.
    pub fn is_inline(&self) -> bool {
        self.display_mode().is_some_and(|mode| mode.is_inline())
    }

    /// Returns `true` if in fullscreen display mode.
    pub fn is_fullscreen(&self) -> bool {
        self.display_mode().is_some_and(|mode| mode.is_fullscreen())
    }

    pub fn fullscreen_conversation_id(&self) -> Option<AIConversationId> {
        match self {
            AgentViewState::Active {
                conversation_id,
                display_mode: AgentViewDisplayMode::FullScreen,
                ..
            } => Some(*conversation_id),
            _ => None,
        }
    }

    pub fn is_new(&self) -> bool {
        match self {
            AgentViewState::Active {
                original_conversation_length,
                ..
            } => *original_conversation_length == 0,
            AgentViewState::Inactive => false,
        }
    }

    /// Returns the save position ID for the zero state block, if active.
    pub fn zero_state_position_id(&self) -> Option<String> {
        self.active_conversation_id()
            .map(|id| format!("agent_view_zero_state_{}", id))
    }
}

pub fn agent_view_bg_fill(app: &AppContext) -> warp_core::ui::theme::Fill {
    Appearance::as_ref(app).theme().surface_overlay_1()
}

pub fn agent_view_bg_color(app: &AppContext) -> ColorU {
    use warp_core::ui::color::blend::Blend;
    agent_view_bg_fill(app)
        .blend(&Appearance::as_ref(app).theme().background())
        .into_solid()
}

pub fn get_agent_view_entry_block_position_id(view_id: warpui::EntityId) -> String {
    format!("agent_view_entry_block_{view_id}")
}

pub fn render_block_container(
    origin: AgentViewEntryOrigin,
    content: Box<dyn Element>,
    background: ColorU,
    appearance: &Appearance,
    are_block_dividers_enabled: bool,
) -> Box<dyn Element> {
    let border = if are_block_dividers_enabled {
        Border::top(1.).with_border_fill(appearance.theme().outline())
    } else {
        Border::new(1.)
            .with_sides(true, false, true, false)
            .with_border_fill(appearance.theme().outline())
    };

    let mut container = Container::new(content).with_background(background);

    if matches!(origin, AgentViewEntryOrigin::LongRunningCommand) {
        container = container
            .with_uniform_padding(12.)
            .with_horizontal_margin(16.)
            .with_margin_bottom(16.)
            .with_margin_top(8.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
    } else {
        container = container
            .with_horizontal_padding(20.)
            .with_vertical_padding(18.)
            .with_border(border);
    }

    container.finish()
}
