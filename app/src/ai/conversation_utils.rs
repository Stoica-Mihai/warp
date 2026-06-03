use warpui::{AppContext, EntityId, SingletonEntity};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent_conversations_model::AgentConversationsModel;

/// Delete a conversation from local storage and the cloud.
pub fn delete_conversation(
    _conversation_id: AIConversationId,
    _terminal_view_id: Option<EntityId>,
    ctx: &mut AppContext,
) {
    AgentConversationsModel::handle(ctx).update(ctx, |model, ctx| {
        model.sync_conversations(ctx);
    });
}
