use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::AIAgentExchangeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResponseStreamId(String);

#[derive(Debug, Clone)]
pub struct ClientIdentifiers {
    pub conversation_id: AIConversationId,
    pub client_exchange_id: AIAgentExchangeId,
    pub response_stream_id: Option<ResponseStreamId>,
}

impl ResponseStreamId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
