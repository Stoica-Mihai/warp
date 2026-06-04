use warpui::UpdateView;

use super::{
    fork_label_for_query, mark_feature_used_and_write_to_user_defaults, AIAgentExchangeId,
    AIConversationId, AppContext, ChannelState,
    ClipboardContent, ContextMenuAction, ContextMenuState, ContextMenuType, EntityId,
    ForkAIConversationParams, ForkFromExchange, ForkedConversationDestination, MenuItem,
    MenuItemFields, RichContentLink,
    TerminalAction, TerminalModel, TerminalView, Tip, TipHint, ViewContext,
    CONTEXT_MENU_WIDTH,
};

impl TerminalView {
    pub(super) fn ai_block_copying_menu_items(
        &self,
        ai_block_view_id: EntityId,
        _ai_conversation_id: AIConversationId,
        hovered_link: Option<RichContentLink>,
        model: &TerminalModel,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut items = vec![
            MenuItemFields::new("Copy")
                .with_on_select_action(TerminalAction::ContextMenu(
                    ContextMenuAction::CopyAIBlock { ai_block_view_id },
                ))
                .into_item(),
            MenuItemFields::new("Copy prompt")
                .with_on_select_action(TerminalAction::ContextMenu(
                    ContextMenuAction::CopyAIBlockQuery { ai_block_view_id },
                ))
                .into_item(),
            MenuItemFields::new("Copy output as Markdown")
                .with_on_select_action(TerminalAction::ContextMenu(
                    ContextMenuAction::CopyAIBlockOutput { ai_block_view_id },
                ))
                .into_item(),
        ];

        if let Some(link) = hovered_link {
            match link {
                RichContentLink::Url(url) => {
                    items.push(
                        MenuItemFields::new("Copy URL")
                            .with_on_select_action(TerminalAction::ContextMenu(
                                ContextMenuAction::CopyUrl { url_content: url },
                            ))
                            .into_item(),
                    );
                }
                #[cfg(feature = "local_fs")]
                RichContentLink::FilePath { absolute_path, .. } => {
                    items.push(
                        MenuItemFields::new("Copy path")
                            .with_on_select_action(TerminalAction::ContextMenu(
                                ContextMenuAction::CopyUrl {
                                    url_content: absolute_path.to_string_lossy().into_owned(),
                                },
                            ))
                            .into_item(),
                    );
                }
            }
        }

        let num_requested_commands = 0;
        let _ = (ai_block_view_id, ctx);

        if num_requested_commands > 0 {
            items.push(
                MenuItemFields::new(String::from("Copy command"))
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::CopyAgentCommand { ai_block_view_id },
                    ))
                    .into_item(),
            );
        }

        let action_ids: Vec<_> = Vec::new();

        let has_git_branch = action_ids.iter().any(|action_id| {
            model
                .block_list()
                .block_for_ai_action_id(action_id)
                .is_some_and(|block| block.git_branch().is_some())
        });
        if has_git_branch {
            items.push(
                MenuItemFields::new(String::from("Copy git branch"))
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::CopyAgentGitBranch { ai_block_view_id },
                    ))
                    .into_item(),
            );
        }
        items.push(MenuItem::Separator);
        items.push(
            MenuItemFields::new("Save as prompt")
                .with_on_select_action(TerminalAction::ContextMenu(
                    ContextMenuAction::SavePromptAsAgentModeWorkflow { ai_block_view_id },
                ))
                .into_item(),
        );
        items.push(MenuItem::Separator);


        items.push(
            MenuItemFields::new("Copy conversation text")
                .with_on_select_action(TerminalAction::ContextMenu(
                    ContextMenuAction::CopyAIBlockConversation { ai_block_view_id },
                ))
                .into_item(),
        );

        items
    }

    fn conversation_text(
        &self,
        _conversation_id: AIConversationId,
        _ctx: &AppContext,
    ) -> Option<String> {
        None
    }

    pub(super) fn copy_conversation_text(
        &self,
        conversation_id: AIConversationId,
        ctx: &mut ViewContext<Self>,
    ) {
        if let Some(conversation_text) = self.conversation_text(conversation_id, ctx) {
            ctx.clipboard()
                .write(ClipboardContent::plain_text(conversation_text));
        }
    }

    pub(super) fn fork_ai_conversation(
        &self,
        conversation_id: AIConversationId,
        fork_from_exchange: Option<ForkFromExchange>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.dispatch_global_action(
            "workspace:fork_ai_conversation",
            ForkAIConversationParams {
                conversation_id,
                fork_from_exchange,
                summarize_after_fork: false,
                summarization_prompt: None,
                initial_prompt: None,
                destination: ForkedConversationDestination::SplitPane,
            },
        );
    }

    pub(super) fn create_copy_debugging_menu_item(
        &self,
        _ai_exchange_id: AIAgentExchangeId,
        _ai_conversation_id: AIConversationId,
        _ctx: &mut ViewContext<Self>,
    ) -> Vec<(String, ContextMenuAction)> {
        Vec::new()
    }

    pub(super) fn open_ai_block_overflow_context_menu(
        &mut self,
        ai_block_view_id: EntityId,
        ai_exchange_id: AIAgentExchangeId,
        ai_conversation_id: AIConversationId,
        _is_restored: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let mut menu_items = {
            let model = self.model.lock();
            self.ai_block_copying_menu_items(
                ai_block_view_id,
                ai_conversation_id,
                None,
                &model,
                ctx,
            )
        };

        if !cfg!(target_family = "wasm") {
            let fork_label = fork_label_for_query(&String::new());
            menu_items.push(
                MenuItemFields::new(fork_label)
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::ForkAIConversationFromBlock {
                            ai_block_view_id,
                            exchange_id: ai_exchange_id,
                            conversation_id: ai_conversation_id,
                        },
                    ))
                    .into_item(),
            );

            if ChannelState::channel().is_dogfood() {
                menu_items.push(
                    MenuItemFields::new("Fork from here")
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::ForkAIConversationFromExactExchange {
                                ai_block_view_id,
                                exchange_id: ai_exchange_id,
                                conversation_id: ai_conversation_id,
                            },
                        ))
                        .into_item(),
                );
            }
        }

        let debugging_items =
            self.create_copy_debugging_menu_item(ai_exchange_id, ai_conversation_id, ctx);
        if !debugging_items.is_empty() {
            if !menu_items.is_empty() {
                menu_items.push(MenuItem::Separator);
            }
            for (button_text, action) in debugging_items {
                menu_items.push(
                    MenuItemFields::new(button_text)
                        .with_on_select_action(TerminalAction::ContextMenu(action))
                        .into_item(),
                );
            }
        }

        self.show_context_menu(
            ContextMenuState {
                menu_type: ContextMenuType::AIBlockOverflowMenu { ai_block_view_id },
            },
            menu_items,
            ctx,
        );
    }

    pub(super) fn show_context_menu(
        &mut self,
        menu_state: ContextMenuState,
        items: Vec<MenuItem<TerminalAction>>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.update_view(&self.context_menu, |context_menu, view_ctx| {
            context_menu.set_origin(menu_state.menu_type.origin());
            context_menu.set_width(CONTEXT_MENU_WIDTH);
            // This will also reset the selection.
            context_menu.set_items(items, view_ctx);
        });

        self.context_menu_state = Some(menu_state);
        ctx.focus(&self.context_menu);
        ctx.notify();

        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });
    }
}
