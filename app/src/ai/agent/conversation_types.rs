//! Lightweight, self-contained agent-conversation types used by surviving
//! features (CLI-agent-sessions status display, conversation identity,
//! server conversation tokens). Kept separate from the heavy `AIConversation`
//! aggregate so the latter can be removed without disturbing these.

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp_core::channel::ChannelState;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::theme::WarpTheme;
use warpui::color::ColorU;

use crate::ai::agent::icons::{
    failed_icon, gray_stop_icon, in_progress_icon, succeeded_icon, yellow_stop_icon,
};
use crate::ui_components::icons::Icon;

/// A globally unique ID for a conversation with an AI agent.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AIConversationId(Uuid);

impl Display for AIConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AIConversationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AIConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<String> for AIConversationId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(Uuid::try_parse(&value)?))
    }
}

/// Unique, server-generated conversation-scoped token to be roundtripped to the API when sending
/// requests that follow-up within a given conversation.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerConversationToken(String);

impl ServerConversationToken {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn debug_link(&self) -> String {
        format!(
            "{}/debug/maa/{}",
            ChannelState::server_root_url(),
            self.as_str()
        )
    }

    pub fn conversation_link(&self) -> String {
        format!(
            "{}/conversation/{}",
            ChannelState::server_root_url(),
            self.as_str()
        )
    }
}

impl From<ServerConversationToken> for String {
    fn from(value: ServerConversationToken) -> Self {
        value.0
    }
}

// Conversions between AI ServerConversationToken and protocol ServerConversationToken
impl From<session_sharing_protocol::common::ServerConversationToken> for ServerConversationToken {
    fn from(token: session_sharing_protocol::common::ServerConversationToken) -> Self {
        Self(token.to_string())
    }
}

impl TryFrom<ServerConversationToken>
    for session_sharing_protocol::common::ServerConversationToken
{
    type Error = uuid::Error;

    fn try_from(token: ServerConversationToken) -> Result<Self, Self::Error> {
        token.as_str().parse()
    }
}

#[derive(Clone, Copy)]
pub enum StatusColorStyle {
    /// Foreground-blend colors (`ansi_fg`) used by the regular status badge.
    Standard,
    /// Background-blend colors (`ansi_bg`) used by the cloud overlay badge.
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConversationStatus {
    /// Agent is running.
    InProgress,

    /// The last turn of the agent finished with success.
    Success,

    /// The last turn of the agent completed with error.
    Error,

    /// The last turn of the agent was cancelled by the user.
    Cancelled,

    /// The last turn of the agent resulted in an action whose execution is blocked by the user.
    Blocked { blocked_action: String },
}

impl std::fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationStatus::InProgress => write!(f, "In progress"),
            ConversationStatus::Success => write!(f, "Done"),
            ConversationStatus::Error => write!(f, "Error"),
            ConversationStatus::Cancelled => write!(f, "Cancelled"),
            ConversationStatus::Blocked { .. } => write!(f, "Blocked"),
        }
    }
}

impl ConversationStatus {
    pub fn render_icon(&self, appearance: &Appearance) -> warpui::elements::Icon {
        match self {
            ConversationStatus::InProgress => in_progress_icon(appearance),
            ConversationStatus::Success => succeeded_icon(appearance),
            ConversationStatus::Blocked { .. } => yellow_stop_icon(appearance),
            ConversationStatus::Error => failed_icon(appearance),
            ConversationStatus::Cancelled => gray_stop_icon(appearance),
        }
    }

    pub fn status_icon_and_color(
        &self,
        theme: &WarpTheme,
        color_style: StatusColorStyle,
    ) -> (Icon, ColorU) {
        match self {
            ConversationStatus::InProgress => (
                Icon::ClockLoader,
                match color_style {
                    StatusColorStyle::Standard => theme.ansi_fg_magenta(),
                    StatusColorStyle::Cloud => theme.ansi_bg_magenta(),
                },
            ),
            ConversationStatus::Success => (
                Icon::Check,
                match color_style {
                    StatusColorStyle::Standard => theme.ansi_fg_green(),
                    StatusColorStyle::Cloud => theme.ansi_bg_green(),
                },
            ),
            ConversationStatus::Error => (
                Icon::Triangle,
                match color_style {
                    StatusColorStyle::Standard => theme.ansi_fg_red(),
                    StatusColorStyle::Cloud => theme.ansi_bg_red(),
                },
            ),
            ConversationStatus::Cancelled => (Icon::StopFilled, internal_colors::neutral_5(theme)),
            ConversationStatus::Blocked { .. } => (
                Icon::StopFilled,
                match color_style {
                    StatusColorStyle::Standard => theme.ansi_fg_yellow(),
                    StatusColorStyle::Cloud => theme.ansi_bg_yellow(),
                },
            ),
        }
    }

    pub fn is_in_progress(&self) -> bool {
        matches!(self, ConversationStatus::InProgress)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, ConversationStatus::Blocked { .. })
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, ConversationStatus::Cancelled)
    }

    pub fn is_done(&self) -> bool {
        matches!(
            self,
            ConversationStatus::Success | ConversationStatus::Error | ConversationStatus::Cancelled
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ConversationStatus::Error)
    }
}
