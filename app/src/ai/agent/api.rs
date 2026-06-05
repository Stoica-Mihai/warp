pub(crate) mod convert_conversation;
mod convert_from;
mod convert_to;
mod r#impl;

pub use convert_from::{
    user_inputs_from_messages, ConversionParams, ConvertAPIMessageToClientOutputMessage,
    MaybeAIAgentOutputMessage, MessageToAIAgentOutputMessageError,
};
use serde::Serialize;
use warp_core::channel::ChannelState;


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


