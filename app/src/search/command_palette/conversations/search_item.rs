use ordered_float::OrderedFloat;
use warp_core::ui::icons::Icon;
use warpui::elements::{
    Container, CrossAxisAlignment, Expanded, Flex, Highlight, MainAxisSize,
    MouseStateHandle, ParentElement, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::{AppContext, Element, SingletonEntity};

use crate::appearance::Appearance;
use crate::search::command_palette::conversations::search::MatchedConversation;
use crate::search::command_palette::mixer::CommandPaletteItemAction;
use crate::search::command_palette::render_util::render_search_item_icon;
use crate::search::item::IconLocation;
use crate::search::result_renderer::ItemHighlightState;
use crate::search::SearchItem;
use crate::util::time_format::format_approx_duration_from_now;

/// Information about which action to take once the conversation item is accepted.
#[derive(Debug)]
pub enum ConversationAction {
    /// Start a new conversation in the current view.
    New,
    /// Resume the matched conversation in its associated view.
    Resume(Box<MatchedConversation>),
}

/// Search item to render a conversation within the command palette.
/// When matched_conversation is None, we render this as a new conversation item.
#[derive(Debug)]
pub struct ConversationSearchItem {
    action_info: ConversationAction,
    action_button_mouse_state: MouseStateHandle,
}

impl ConversationSearchItem {
    pub fn new(action_info: ConversationAction) -> Self {
        Self {
            action_info,
            action_button_mouse_state: MouseStateHandle::default(),
        }
    }

    /// Renders the new conversation item for the command palette.
    pub fn render_new_conversation_action_item(
        &self,
        highlight_state: ItemHighlightState,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        Flex::row()
            .with_child(
                Text::new_inline(
                    "New conversation",
                    appearance.ui_font_family(),
                    appearance.monospace_font_size(),
                )
                .with_color(highlight_state.sub_text_fill(appearance).into_solid())
                .with_style(Properties::default().weight(Weight::Bold))
                .finish(),
            )
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .finish()
    }

    fn render_matched_conversation_item(
        &self,
        matched_conversation: &MatchedConversation,
        highlight_state: ItemHighlightState,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let conversation = matched_conversation.conversation.clone();
        let sub_text_font_size = appearance.monospace_font_size() - 2.;

        let mut conversation_title_element = Text::new_inline(
            conversation.title().to_string(),
            appearance.ui_font_family(),
            appearance.monospace_font_size(),
        )
        .with_color(highlight_state.sub_text_fill(appearance).into_solid())
        .with_style(Properties::default().weight(Weight::Bold));

        let mut working_directory_element = Text::new_inline(
            conversation
                .initial_working_directory
                .clone()
                .unwrap_or_default(),
            appearance.ui_font_family(),
            sub_text_font_size,
        )
        .with_color(highlight_state.sub_text_fill(appearance).into_solid());

        // When the search query is empty, we only show the conversation's title and working directory.
        // Otherwise, we show the conversation's title, initial user query, and working directory.
        // We also highlight the indices in those elements that match the search query.
        let mut left_container = Flex::column().with_spacing(4.);
        if !self.query_is_empty() {
            // The first user query that was submitted for this conversation.
            let mut initial_query_element = Text::new_inline(
                conversation.initial_query.clone().unwrap_or_default(),
                appearance.ui_font_family(),
                sub_text_font_size,
            )
            .with_color(highlight_state.sub_text_fill(appearance).into_solid());

            // Apply highlights for the search query's matching indices.
            let highlight = Highlight::new()
                .with_properties(Properties::default().weight(Weight::Bold))
                .with_foreground_color(highlight_state.main_text_fill(appearance).into_solid());
            let highlight_indices = matched_conversation.highlight_indices();
            if !highlight_indices.title_indices().is_empty() {
                conversation_title_element = conversation_title_element
                    .with_single_highlight(highlight, highlight_indices.title_indices().clone());
            }
            if !highlight_indices.initial_query_indices().is_empty() {
                initial_query_element = initial_query_element.with_single_highlight(
                    highlight,
                    highlight_indices.initial_query_indices().clone(),
                );
            }
            if !highlight_indices.working_directory_indices().is_empty() {
                working_directory_element = working_directory_element.with_single_highlight(
                    highlight,
                    highlight_indices.working_directory_indices().clone(),
                );
            }

            // Add the conversation title and initial user query to the left container.
            left_container = left_container
                .with_child(conversation_title_element.finish())
                .with_child(initial_query_element.finish());
        } else {
            // When the search query is empty, we only show the conversation's title and working directory.
            left_container = left_container.with_child(conversation_title_element.finish());
        }
        // In all cases, we show the conversation's working directory last.
        left_container = left_container.with_child(working_directory_element.finish());

        let last_updated = format_approx_duration_from_now(conversation.last_updated());
        let last_updated_element = Container::new(
            Text::new_inline(
                last_updated,
                appearance.ui_font_family(),
                sub_text_font_size,
            )
            .with_color(highlight_state.sub_text_fill(appearance).into_solid())
            .finish(),
        )
        .with_padding_left(8.)
        .finish();

        let search_item_content = Flex::row()
            .with_child(Expanded::new(1.0, left_container.finish()).finish())
            .with_child(last_updated_element)
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        search_item_content
    }

    fn query_is_empty(&self) -> bool {
        match &self.action_info {
            ConversationAction::Resume(matched_conversation) => {
                // If the score is empty, the query must be empty (otherwise, we would not be showing this item)
                matched_conversation.as_ref().match_result.score() == 0
            }
            ConversationAction::New => {
                // We only show these items when the search query is empty.
                true
            }
        }
    }
}

impl SearchItem for ConversationSearchItem {
    type Action = CommandPaletteItemAction;

    fn is_multiline(&self) -> bool {
        true
    }

    fn render_icon(
        &self,
        highlight_state: ItemHighlightState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let (color, icon) = match &self.action_info {
            ConversationAction::Resume(..) => (
                appearance.theme().foreground().into_solid(),
                Icon::Conversation,
            ),
            ConversationAction::New => (appearance.theme().foreground().into_solid(), Icon::Plus),
        };

        render_search_item_icon(appearance, icon, color, highlight_state)
    }

    fn icon_location(&self, appearance: &Appearance) -> IconLocation {
        if matches!(self.action_info, ConversationAction::New) {
            IconLocation::Centered
        } else {
            // The icon has the size of the monospace font, whereas the text has a height of
            // `line_height_ratio * font_size`. Offset the icon by this difference so it is rendered
            // centered with the text.
            let margin_top = (appearance.line_height_ratio() * appearance.monospace_font_size())
                - appearance.monospace_font_size();
            IconLocation::Top { margin_top }
        }
    }

    fn render_item(
        &self,
        highlight_state: ItemHighlightState,
        app: &AppContext,
    ) -> Box<dyn Element> {
        match &self.action_info {
            ConversationAction::Resume(matched_conversation) => self
                .render_matched_conversation_item(
                    matched_conversation.as_ref(),
                    highlight_state,
                    app,
                ),
            ConversationAction::New => {
                self.render_new_conversation_action_item(highlight_state, app)
            }
        }
    }

    fn score(&self) -> OrderedFloat<f64> {
        let score = match &self.action_info {
            ConversationAction::Resume(matched_conversation) => matched_conversation.score() as f64,
            ConversationAction::New => f64::NAN,
        };
        OrderedFloat::from(score)
    }

    fn accept_result(&self) -> Self::Action {
        match &self.action_info {
            ConversationAction::Resume(matched_conversation) => {
                let conversation = &matched_conversation.as_ref().conversation;
                CommandPaletteItemAction::NavigateToConversation {
                    pane_view_locator: conversation.pane_view_locator(),
                    window_id: conversation.window_id(),
                    conversation_id: conversation.id(),
                    terminal_view_id: conversation.terminal_view_id,
                }
            }
            ConversationAction::New => CommandPaletteItemAction::NewConversation,
        }
    }

    fn execute_result(&self) -> Self::Action {
        self.accept_result()
    }

    fn accessibility_label(&self) -> String {
        match &self.action_info {
            ConversationAction::Resume(matched_conversation) => {
                format!(
                    "Conversation: {}",
                    matched_conversation.as_ref().conversation.title()
                )
            }
            ConversationAction::New => "New conversation".to_string(),
        }
    }

    fn accessibility_help_message(&self) -> Option<String> {
        match &self.action_info {
            ConversationAction::Resume(matched_conversation) => Some(format!(
                "Press enter to navigate to conversation \"{}\".",
                matched_conversation.as_ref().conversation.title()
            )),
            ConversationAction::New => Some("Press enter to create a new conversation.".into()),
        }
    }
}
