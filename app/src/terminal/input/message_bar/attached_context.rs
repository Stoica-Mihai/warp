//! Shared message producers for displaying attached blocks/text context.

use crate::ai::blocklist::{BlocklistAIContextModel, BlocklistAIInputModel};
use crate::terminal::input::buffer_model::InputBufferModel;
use crate::terminal::input::message_bar::{
    truncated_command_for_block, Message, MessageItem, MessageProvider,
};
use crate::terminal::model::TerminalModel;

/// Trait for message args that can provide attached context information.
/// Exposes the required dependencies for attached context message producers.
pub trait AttachedContextArgs {
    fn terminal_model(&self) -> &TerminalModel;
    fn input_buffer_model(&self) -> &InputBufferModel;
    fn input_model(&self) -> &BlocklistAIInputModel;
    fn context_model(&self) -> &BlocklistAIContextModel;
}

/// Produces a message when blocks or selected text are attached as context.
pub struct AttachedBlocksMessageProducer;

impl<Args: AttachedContextArgs + Copy> MessageProvider<Args> for AttachedBlocksMessageProducer {
    fn produce_message(&self, args: Args) -> Option<Message> {
        let context_block_ids = args.context_model().pending_context_block_ids();
        if context_block_ids.is_empty() {
            return None;
        }

        let block_command = context_block_ids
            .iter()
            .find_map(|id| {
                args.terminal_model()
                    .block_list()
                    .block_with_id(id)
                    .map(|block| block.command_to_string())
            })
            .map(|cmd| truncated_command_for_block(&cmd))?;

        let message_text = if context_block_ids.len() == 1 {
            format!("`{}` attached as context", block_command)
        } else if context_block_ids.len() == 2 {
            format!(
                "`{}` and 1 other command attached as context",
                block_command
            )
        } else {
            format!(
                "`{}` and {} other commands attached as context",
                block_command,
                context_block_ids.len().saturating_sub(1)
            )
        };

        Some(Message::new(vec![MessageItem::text(message_text)]))
    }
}

/// Produces a message when text selection is attached as context.
pub struct AttachedTextSelectionMessageProducer;

impl<Args: AttachedContextArgs + Copy> MessageProvider<Args>
    for AttachedTextSelectionMessageProducer
{
    fn produce_message(&self, args: Args) -> Option<Message> {
        // Only show if there's selected text and no blocks attached
        // (blocks take precedence per requirements)
        if !args.context_model().pending_context_block_ids().is_empty() {
            return None;
        }

        let _ = args.context_model().pending_context_selected_text()?;

        Some(Message::new(vec![MessageItem::text(
            "selected text attached as context",
        )]))
    }
}
