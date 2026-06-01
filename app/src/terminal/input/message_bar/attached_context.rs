//! Shared message producers for displaying attached blocks/text context.

use crate::ai::blocklist::BlocklistAIInputModel;
use crate::terminal::input::buffer_model::InputBufferModel;
use crate::terminal::input::message_bar::{Message, MessageProvider};
use crate::terminal::model::TerminalModel;

/// Trait for message args that can provide attached context information.
/// Exposes the required dependencies for attached context message producers.
pub trait AttachedContextArgs {
    fn terminal_model(&self) -> &TerminalModel;
    fn input_buffer_model(&self) -> &InputBufferModel;
    fn input_model(&self) -> &BlocklistAIInputModel;
}

/// Produces a message when blocks or selected text are attached as context.
pub struct AttachedBlocksMessageProducer;

impl<Args: AttachedContextArgs + Copy> MessageProvider<Args> for AttachedBlocksMessageProducer {
    fn produce_message(&self, _args: Args) -> Option<Message> {
        None
    }
}

/// Produces a message when text selection is attached as context.
pub struct AttachedTextSelectionMessageProducer;

impl<Args: AttachedContextArgs + Copy> MessageProvider<Args>
    for AttachedTextSelectionMessageProducer
{
    fn produce_message(&self, _args: Args) -> Option<Message> {
        None
    }
}
