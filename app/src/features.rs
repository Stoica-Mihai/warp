use std::collections::HashSet;

use warp_core::channel::ChannelState;
pub use warp_core::features::*;

/// Mark all features which should be enabled on the current channel as enabled.
/// This sets global feature flag state and should never be called in a unit test.
pub fn init_feature_flags() {
    for flag in enabled_features() {
        flag.set_enabled(true);
    }
    mark_initialized();
}

/// Returns all feature flags which should be enabled in the current channel.
fn enabled_features() -> HashSet<FeatureFlag> {
    // Enable features overridden for the given channel.
    let mut flags = ChannelState::additional_features();

    // Enable flags for release builds, if appropriate.
    if ChannelState::is_release_bundle() {
        flags.extend(RELEASE_FLAGS);
    }

    flags.extend([
        #[cfg(feature = "runtime_feature_flags")]
        FeatureFlag::RuntimeFeatureFlags,
        #[cfg(feature = "sequential_storage")]
        FeatureFlag::SequentialStorage,
        #[cfg(feature = "in_band_generators_ssh")]
        FeatureFlag::InBandGeneratorsForSSH,
        #[cfg(feature = "run_generators_with_cmd_exe")]
        FeatureFlag::RunGeneratorsWithCmdExe,
        #[cfg(feature = "ligatures")]
        FeatureFlag::Ligatures,
        #[cfg(feature = "selectable_prompt")]
        FeatureFlag::SelectablePrompt,
        #[cfg(feature = "agent_mode")]
        FeatureFlag::AgentMode,
        #[cfg(feature = "resize_fix")]
        FeatureFlag::ResizeFix,
        #[cfg(feature = "richtext_multiselect")]
        FeatureFlag::RichTextMultiselect,
        #[cfg(feature = "settings_file")]
        FeatureFlag::SettingsFile,
        #[cfg(feature = "rect_selection")]
        FeatureFlag::RectSelection,
        #[cfg(feature = "alacritty_settings_import")]
        FeatureFlag::AlacrittySettingsImport,
        #[cfg(feature = "dynamic_workflow_enums")]
        FeatureFlag::DynamicWorkflowEnums,
        #[cfg(feature = "shared_with_me")]
        FeatureFlag::SharedWithMe,
        #[cfg(feature = "am_workflows")]
        FeatureFlag::AgentModeWorkflows,
        #[cfg(feature = "ai_rules")]
        FeatureFlag::AIRules,
        #[cfg(feature = "ssh_tmux_wrapper")]
        FeatureFlag::SSHTmuxWrapper,
        #[cfg(feature = "shell_selector")]
        FeatureFlag::ShellSelector,
        #[cfg(feature = "artifact_command")]
        FeatureFlag::ArtifactCommand,
        #[cfg(feature = "full_screen_zen_mode")]
        FeatureFlag::FullScreenZenMode,
        #[cfg(feature = "minimalist_ui")]
        FeatureFlag::MinimalistUI,
        #[cfg(feature = "avatar_in_tab_bar")]
        FeatureFlag::AvatarInTabBar,
        #[cfg(feature = "workflow_aliases")]
        FeatureFlag::WorkflowAliases,
        #[cfg(feature = "ssh_drag_and_drop")]
        FeatureFlag::SshDragAndDrop,
        #[cfg(feature = "drag_tabs_to_windows")]
        FeatureFlag::DragTabsToWindows,
        #[cfg(feature = "cycle_next_command_suggestion")]
        FeatureFlag::CycleNextCommandSuggestion,
        #[cfg(feature = "multi_workspace")]
        FeatureFlag::MultiWorkspace,
        #[cfg(feature = "ime_marked_text")]
        FeatureFlag::ImeMarkedText,
        #[cfg(feature = "iterm_images")]
        FeatureFlag::ITermImages,
        #[cfg(feature = "prompt_suggestions_via_maa")]
        FeatureFlag::PromptSuggestionsViaMAA,
        #[cfg(feature = "clear_autosuggestion_on_escape")]
        FeatureFlag::ClearAutosuggestionOnEscape,
        #[cfg(all(not(windows), feature = "kitty_images"))]
        FeatureFlag::KittyImages,
        #[cfg(feature = "default_adeberry_theme")]
        FeatureFlag::DefaultAdeberryTheme,
        #[cfg(feature = "command_correction_key")]
        FeatureFlag::CommandCorrectionKey,
        #[cfg(feature = "full_source_code_embedding")]
        FeatureFlag::FullSourceCodeEmbedding,
        #[cfg(feature = "remote_codebase_indexing")]
        FeatureFlag::RemoteCodebaseIndexing,
        #[cfg(feature = "use_tantivy_search")]
        FeatureFlag::UseTantivySearch,
        #[cfg(feature = "mcp_server")]
        FeatureFlag::McpServer,
        #[cfg(feature = "mcp_debugging_ids")]
        FeatureFlag::McpDebuggingIds,
        #[cfg(feature = "markdown_tables")]
        FeatureFlag::MarkdownTables,
        #[cfg(feature = "blocklist_markdown_table_rendering")]
        FeatureFlag::BlocklistMarkdownTableRendering,
        #[cfg(feature = "markdown_mermaid")]
        FeatureFlag::MarkdownMermaid,
        #[cfg(feature = "editable_markdown_mermaid")]
        FeatureFlag::EditableMarkdownMermaid,
        #[cfg(feature = "image_as_context")]
        FeatureFlag::ImageAsContext,
        #[cfg(feature = "msys2_shells")]
        FeatureFlag::MSYS2Shells,
        #[cfg(feature = "read_image_files")]
        FeatureFlag::ReadImageFiles,
        #[cfg(feature = "codebase_index_persistence")]
        FeatureFlag::CodebaseIndexPersistence,
        #[cfg(feature = "ai_context_menu")]
        FeatureFlag::AIContextMenuEnabled,
        #[cfg(feature = "at_menu_outside_of_ai_mode")]
        FeatureFlag::AtMenuOutsideOfAIMode,
        #[cfg(feature = "agent_decides_command_execution")]
        FeatureFlag::AgentDecidesCommandExecution,
        #[cfg(feature = "context_line_review_comments")]
        FeatureFlag::ContextLineReviewComments,
        #[cfg(feature = "fast_forward_autoexecute_button")]
        FeatureFlag::FastForwardAutoexecuteButton,
        #[cfg(feature = "code_find_replace")]
        FeatureFlag::CodeFindReplace,
        #[cfg(feature = "command_palette_file_search")]
        FeatureFlag::CommandPaletteFileSearch,
        #[cfg(feature = "ai_context_menu_commands")]
        FeatureFlag::AIContextMenuCommands,
        #[cfg(feature = "ai_context_menu_code")]
        FeatureFlag::AIContextMenuCode,
        #[cfg(feature = "tab_close_button_on_left")]
        FeatureFlag::TabCloseButtonOnLeft,
        #[cfg(feature = "profiles_design_revamp")]
        FeatureFlag::ProfilesDesignRevamp,
        #[cfg(feature = "linked_code_blocks")]
        FeatureFlag::LinkedCodeBlocks,
        #[cfg(feature = "tabbed_editor_view")]
        FeatureFlag::TabbedEditorView,
        #[cfg(feature = "undo_closed_panes")]
        FeatureFlag::UndoClosedPanes,
        #[cfg(feature = "projects")]
        FeatureFlag::Projects,
        #[cfg(feature = "drive_objects_as_context")]
        FeatureFlag::DriveObjectsAsContext,
        #[cfg(feature = "pr_comments_slash_command")]
        FeatureFlag::PRCommentsSlashCommand,
        #[cfg(feature = "pr_comments_skill")]
        FeatureFlag::PRCommentsSkill,
        #[cfg(feature = "selection_as_context")]
        FeatureFlag::SelectionAsContext,
        #[cfg(feature = "github_pr_prompt_chip")]
        FeatureFlag::GithubPrPromptChip,
        #[cfg(feature = "vim_code_editor")]
        FeatureFlag::VimCodeEditor,
        #[cfg(feature = "allow_opening_file_links_using_editor_env")]
        FeatureFlag::AllowOpeningFileLinksUsingEditorEnv,
        #[cfg(feature = "revert_diff_hunk")]
        FeatureFlag::RevertDiffHunk,
        #[cfg(feature = "code_review_save_changes")]
        FeatureFlag::CodeReviewSaveChanges,
        #[cfg(feature = "file_tree")]
        FeatureFlag::FileTree,
        #[cfg(feature = "allow_ignoring_input_suggestions")]
        FeatureFlag::AllowIgnoringInputSuggestions,
        #[cfg(feature = "code_launch_modal")]
        FeatureFlag::CodeLaunchModal,
        #[cfg(feature = "api_key_authentication")]
        FeatureFlag::APIKeyAuthentication,
        #[cfg(feature = "mcp_oauth")]
        FeatureFlag::McpOauth,
        #[cfg(feature = "file_based_mcp")]
        FeatureFlag::FileBasedMcp,
        #[cfg(feature = "diff_set_as_context")]
        FeatureFlag::DiffSetAsContext,
        #[cfg(feature = "discard_per_file_and_all_changes")]
        FeatureFlag::DiscardPerFileAndAllChanges,
        #[cfg(feature = "code_review_find")]
        FeatureFlag::CodeReviewFind,
        #[cfg(feature = "ui_zoom")]
        FeatureFlag::UIZoom,
        #[cfg(feature = "auto_open_code_review_pane")]
        FeatureFlag::AutoOpenCodeReviewPane,
        #[cfg(feature = "inline_code_review")]
        FeatureFlag::InlineCodeReview,
        #[cfg(feature = "mcp_grouped_server_context")]
        FeatureFlag::MCPGroupedServerContext,
        #[cfg(feature = "global_search")]
        FeatureFlag::GlobalSearch,
        #[cfg(feature = "embedded_code_review_comments")]
        FeatureFlag::EmbeddedCodeReviewComments,
        #[cfg(feature = "file_and_diff_set_comments")]
        FeatureFlag::FileAndDiffSetComments,
        #[cfg(feature = "agent_management_view")]
        FeatureFlag::AgentManagementView,
        #[cfg(feature = "agent_view")]
        FeatureFlag::AgentView,
        #[cfg(feature = "v4a_file_diffs")]
        FeatureFlag::V4AFileDiffs,
        #[cfg(feature = "interactive_conversation_management_view")]
        FeatureFlag::InteractiveConversationManagementView,
        #[cfg(feature = "agent_mode_computer_use")]
        FeatureFlag::AgentModeComputerUse,
        #[cfg(feature = "local_computer_use")]
        FeatureFlag::LocalComputerUse,
        #[cfg(feature = "local_claude_codex_child_harnesses")]
        FeatureFlag::LocalClaudeCodexChildHarnesses,
        #[cfg(feature = "cloud_conversations")]
        FeatureFlag::CloudConversations,
        #[cfg(feature = "configurable_toolbar")]
        FeatureFlag::ConfigurableToolbar,
        #[cfg(feature = "classic_completions")]
        FeatureFlag::ClassicCompletions,
        #[cfg(feature = "force_classic_completions")]
        FeatureFlag::ForceClassicCompletions,
        #[cfg(feature = "inline_history_menu")]
        FeatureFlag::InlineHistoryMenu,
        #[cfg(feature = "inline_repo_menu")]
        FeatureFlag::InlineRepoMenu,
        #[cfg(feature = "cloud_mode")]
        FeatureFlag::CloudMode,
        #[cfg(feature = "summarization_via_message_replacement")]
        FeatureFlag::SummarizationViaMessageReplacement,
        #[cfg(feature = "pluggable_notifications")]
        FeatureFlag::PluggableNotifications,
        #[cfg(feature = "async_find")]
        FeatureFlag::AsyncFind,
        #[cfg(feature = "list_skills")]
        FeatureFlag::ListSkills,
        #[cfg(feature = "ask_user_question")]
        FeatureFlag::AskUserQuestion,
        #[cfg(feature = "bundled_skills")]
        FeatureFlag::BundledSkills,
        #[cfg(feature = "new_tab_styling")]
        FeatureFlag::NewTabStyling,
        #[cfg(feature = "conversations_as_context")]
        FeatureFlag::ConversationsAsContext,
        #[cfg(feature = "incremental_auto_reload")]
        FeatureFlag::IncrementalAutoReload,
        #[cfg(feature = "orchestration_v2")]
        FeatureFlag::OrchestrationV2,
        #[cfg(feature = "orchestration_viewer_pill_bar")]
        FeatureFlag::OrchestrationViewerPillBar,
        #[cfg(feature = "run_agents_tool")]
        FeatureFlag::RunAgentsTool,
        #[cfg(feature = "queue_slash_command")]
        FeatureFlag::QueueSlashCommand,
        #[cfg(feature = "kitty_keyboard_protocol")]
        FeatureFlag::KittyKeyboardProtocol,
        #[cfg(feature = "inline_menu_headers")]
        FeatureFlag::InlineMenuHeaders,
        #[cfg(feature = "directory_tab_colors")]
        FeatureFlag::DirectoryTabColors,
        #[cfg(feature = "open_warp_new_settings_modes")]
        FeatureFlag::OpenWarpNewSettingsModes,
        #[cfg(feature = "hoa_code_review")]
        FeatureFlag::HoaCodeReview,
        #[cfg(feature = "vertical_tabs")]
        FeatureFlag::VerticalTabs,
        #[cfg(feature = "vertical_tabs_summary_mode")]
        FeatureFlag::VerticalTabsSummaryMode,
        #[cfg(feature = "tab_configs")]
        FeatureFlag::TabConfigs,
        #[cfg(feature = "agent_harness")]
        FeatureFlag::AgentHarness,
        #[cfg(feature = "oz_handoff")]
        FeatureFlag::OzHandoff,
        #[cfg(feature = "handoff_local_cloud")]
        FeatureFlag::HandoffLocalCloud,
        #[cfg(feature = "hoa_notifications")]
        FeatureFlag::HOANotifications,
        #[cfg(feature = "open_code_notifications")]
        FeatureFlag::OpenCodeNotifications,
        #[cfg(feature = "cli_agent_rich_input")]
        FeatureFlag::CLIAgentRichInput,
        #[cfg(feature = "transfer_control_tool")]
        FeatureFlag::TransferControlTool,
        #[cfg(feature = "solo_user_byok")]
        FeatureFlag::SoloUserByok,
        #[cfg(feature = "git_operations_in_code_review")]
        FeatureFlag::GitOperationsInCodeReview,
        #[cfg(feature = "codex_notifications")]
        FeatureFlag::CodexNotifications,
        #[cfg(feature = "trim_trailing_blank_lines")]
        FeatureFlag::TrimTrailingBlankLines,
        #[cfg(feature = "cloud_mode_setup_v2")]
        FeatureFlag::CloudModeSetupV2,
        #[cfg(feature = "cloud_mode_input_v2")]
        FeatureFlag::CloudModeInputV2,
        #[cfg(feature = "configurable_context_window")]
        FeatureFlag::ConfigurableContextWindow,
        #[cfg(feature = "handoff_cloud_cloud")]
        FeatureFlag::HandoffCloudCloud,
        #[cfg(feature = "remote_code_review")]
        FeatureFlag::RemoteCodeReview,
        #[cfg(feature = "custom_inference_endpoints")]
        FeatureFlag::CustomInferenceEndpoints,
    ]);

    flags
}
