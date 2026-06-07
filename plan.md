# Warp cloud-strip — removal plan

Branch: `surgical-strip` off upstream `0b737e22` (green baseline: `cargo check -p warp` = 0 errors, 129 crates).
Invariant: **tree stays green after every commit** — never delete a dependency before its callers are refactored off it.

Difficulty rubric:
- **EASY** — self-contained crate/module, fan-in <10 files, not in render path, or fully feature-gated leaf.
- **MEDIUM** — fan-in 10–50 files, surgical edits to mixed files, not render-core.
- **HARD** — fan-in >50 files, OR woven into the render path (`terminal/view.rs`, `terminal/input.rs`, `pane_group/mod.rs`, `root_view.rs`, `workspace/view.rs`), OR inside core engine crate `warp_core`.

Baseline sizes (measured): app/src = 2073 files / 1.0M LoC. AI = 217k LoC (22%).

**Landing-screen principle**: a welcome/landing-screen action is removable if it's also reachable via settings, the command palette, or default behavior. Removing a landing surface (welcome / get-started) means re-pointing new-tab creation at a plain terminal pane (`LeafContents::Terminal`) — a MEDIUM behavior change, not a clean file delete.

**Note on investigator tags**: difficulty/category come from measured fan-in, but a few name-based "useless" guesses didn't survive reading the code (e.g. `welcome_palette` is the new-tab landing surface, not a throwaway). Treat §4 EASY items as "verify by reading before deleting."

---

## Status / progress log

Legend: ✅ done · 🔄 in progress · ⏸️ back-burner · ⬜ todo · 🔍 needs verify

| Item | Status | Commit | Notes |
|---|---|---|---|
| Welcome + get-started landing **panes** (`welcome_palette`, `welcome_view`, `get_started_view/pane`) | ✅ done · ✅ verified | `59562bd7` | New tab defaults to terminal; `LeafContents::{Welcome,GetStarted}` + palette removed; `welcome_panes` sqlite table dropped. |
| Onboarding experience (app-side flow) | ✅ done · ✅ verified | `3e87396f` | App launches straight to terminal — the "Welcome to Warp/Get started/Log in" screen was the onboarding intro slide, now gone. `crates/onboarding` retained (login_slide); crate-delete = login pass. 87 warnings + 3 no-op stubs to clean. |
| Orphaned get-started sub-views (`coding_entrypoints/`: `clone_repo_view`, `create_project_view`, `project_buttons`) | ✅ done | `3e87396f` | Already deleted as part of the onboarding-flow strip (pickaxe confirms `project_buttons`/`coding_entrypoints` symbols vanish at `3e87396f`/`a2264b69`). Zero tracked refs remain (session 61 verify). The "still called in lib.rs:1605" note was stale — that line is now a window-close handler. |
| Onboarding app flow | ✅ done (`3e87396f`) | — | Login KEPT. `crates/onboarding` retained (login_slide seam) → delete in login pass. |
| Settings-import regression (over-stubbed in onboarding pass) | ✅ done | `e9a3dc2e` | `ImportSettings` action was no-op'd; restored standalone `add_settings_import_block` (no onboarding chaining/telemetry). Triggered by `HAS_SETTINGS_TO_IMPORT_FLAG` (config detection, `local_fs`), NOT onboarding. Green default + `--features local_fs`. |
| Onboarding dead-code cleanup (action wiring + helpers) | ✅ done | `d444c2b6` | Removed `TerminalAction::{OnboardingFlow,SelectAgenticSuggestion}` + `OnboardingVersion`/`AgentOnboardingVersion` enums, 11 onboarding keybindings, 5 dead helper methods, `onboarding_theme_picker_themes`. Warnings 86→79. `OnboardingIntention` kept (login/oz_launch/workspace still use it). |
| Telemetry — step 1: no-op all 5 send-macros | ✅ done | `4b2d993f` | 782 call-sites inert; macros discard args so enum could be deleted without touching them. |
| Telemetry — step 2: remove live send path | ✅ done | `a8ae755c` | Direct `SessionAbandonedBeforeBootstrap` leak + callerless `ServerApi::send_telemetry_event` gone. Nothing phones home (queue never filled). |
| Telemetry — step 3: delete `TelemetryEvent` enum + all construction | ✅ done | `a9b89f9b` | −6966 LoC. Enum + discriminants + impls + events_tests gone; 127 imports swept (recast, crate::-anchored); ~33 real-code sites (From/TryFrom impls, page-builders, struct fields `AIAgentOutput.telemetry_events`/`CTAButton.telemetry_event`, `let event=…;send!` sites, direct `record_event(Login)`). Green on all 4 gates. Payload structs in events.rs KEPT (used by real code). |
| Telemetry — step a: all 16 satellite `*TelemetryEvent` enums | ✅ done | `e466235a` `2eebd0fe` `8921cfcb` | Batch 1: 4 pure. Batch 2: 9 mixed (surgical — keep helper types real code uses). Batch 3: 3 crate-level (ai/onboarding/repo_metadata). Helpers kept (events.rs pattern); construction was in no-op'd macro args, only re-export chains + builder fns needed fixing. |
| Telemetry — step e: warning sweep | ✅ partial | `bcd1fc6a` | `cargo fix --features gui,local_fs` swept unused imports/vars **795 → ~142**. Remaining ~142 = `dead_code` on never-constructed payload structs in `events.rs` (die with b). ⚠ `cargo fix` under one feature set drops `#[cfg(test)]`/other-feature-only imports → re-verify all 4 gates (it broke `--tests`). |
| Telemetry — step b: warp_core trait machinery | ✅ done | `081a425a` | Deleted `TelemetryEvent`/`TelemetryEventDesc`/`RegisteredTelemetryEvent` traits + `register_telemetry_event!` + `enum_events` + `all_events` + `EnablementState` enum + dead `TelemetryApi::send_telemetry_event` method. Swept 11 orphan trait imports. −273 LoC. |
| Telemetry — step c: app dispatch infra (8 commits) | ✅ done | `7c2fe36b` `b7ba00d9` `dec01756` `2ea55e2d` `5f1d030b` `879ef26b` `106f62e3` `3dadef72` | c1-c8: TelemetryCollector entry points (lib.rs + auth_manager) → collector module → ServerApi flush/persist surface → TelemetryApi + rudder_message + telemetry_ext + mod_tests → secret_redaction → context.rs + shared_session telemetry_context → AppTelemetryContextProvider + TelemetryContext{Model,Provider} → warpui_core record layer + app_focus_telemetry. Includes step d (lib.rs:283/1545/1546 bootstrap was in c1). −2579 LoC. |
| Telemetry — cleanup: 819 macro call-sites + 5 macro defs | ✅ done | `a052b2a3` | recast deleted 819 `send_telemetry_*!(...)` invocations across ~178 files (754 + 38 + 9 + 12 + 6). cargo fix swept orphan imports across `warp`/`ai`/`onboarding`/`repo_metadata`. Hand-fixed: recast leftovers (`crate::\n`, `crate::    }`), pub re-exports, Windows-only imports. Deleted 5 no-op macro defs + parent module decls. −5802 LoC across 193 files. |
| Telemetry — cleanup: 43 dead events.rs payload structs | ✅ done | `c7319c0a` | Script-driven pruning of items with zero external refs + 3 cascade-deleted orphan `impl From<X> for DELETED` blocks. events.rs: 1163 → 626 lines. **Telemetry strip = COMPLETE.** GUI binary builds (`cargo build --bin warp-oss --features gui` = 0 errors, 914 MB). |
| Wasm-orphan crates: `serve-wasm`, `managed_secrets_wasm` | ✅ done | `029a7e17` | Pure orphans (zero fan-in). −281 LoC. Removed `default-members` carve-out comment for `serve-wasm`. |
| Wasm-orphan crate: `warp_web_event_bus` | ✅ done | `7faa67bb` | Two consumers, both inside `cfg(target_family = "wasm")` shims: `app/src/platform/wasm.rs` re-export dropped; `crates/warp_logging/src/wasm.rs` `WasmLogger` error-log Sentry hop collapsed to plain `console::error_4`. Crate + workspace + Cargo entries gone. −117 LoC (+ 6 churn). GUI binary unchanged (wasm crates were never linked into the linux build). |
| Login gate bypass | ✅ done | `ecf81c67` | `root_view::new()` always `AuthOnboardingState::Terminal`; `lib.rs` first-frame callback unconditional; `refresh_user` + `download_method` analytics branch dropped; `crash_recovery::on_frame_drawn` path deleted; `DownloadSource` + `download_method.rs` deleted. −212 LoC. 3-gate 0/0/0. |
| Auth gate UI strip | ✅ done | `2e8a10a3` | `RootView.auth_onboarding_state: AuthOnboardingState` → `workspace: ViewHandle<Workspace>`. Deleted: `AuthOnboardingState`/`AuthOnboardingTarget` enums + impls; `WorkspaceArgs`; `handle_auth_manager_event` + gate handler tree; `needs_sso_link_view.rs`, `login_error_modal.rs`, `web_handoff.rs`; `root_view_tests.rs`. Cascaded: `AuthViewVariant::Initial` removal → loginless/overlay flow in `auth_view_body.rs` pruned; `auth_view_shared_helpers.rs` stripped to 2 fns; `set_user_onboarded`/`link_sso_url`/`create_anonymous_user`/`SkippedLogin`/`set_user_is_onboarded` chain gone. 13 files, −2,287 LoC. 3-gate 0/0/0. KEPT: `AuthView`/`AuthOverrideWarningModal` (used by workspace/view.rs for cloud-feature login-gate triggers — defer to auth_manager pass). |
| AuthManager login stub | ✅ done | `721f5ebf` | `auth_manager.rs` drops `auth_client` + `server_api` fields; `new()` takes only `ctx`. Deleted: `initialize_user_from_auth_payload`, `resume_interrupted_auth_payload`, `on_user_fetched`, `on_device_code_received`, `login_options_url`, `copy_anonymous_user_linking_url_to_clipboard`, `consume_auth_state`, `should_silently_ignore_stale_redirect`. No-op'd: `refresh_user`, `authorize_device`, `initiate_anonymous_user_linking`. `open_url_maybe_with_anonymous_token` → always opens without token. `AuthManagerEvent` collapsed to `NeedsReauth` + `AttemptedLoginGatedFeature`. Cascade deleted: `AuthComplete`/`AuthFailed`/`ReceivedDeviceAuth` subscriber arms across 7 AI+auth files; dead `one_time_modal` launch-modal methods; dead workspace telemetry-banner + open_auth_override methods; `FetchUserResult`, `UserProperties`, `MintCustomTokenError`, `exchange_credentials`, dead `AuthClient` trait methods (`fetch_user`, `fetch_new_custom_token`, `on_custom_token_fetched`, `fetch_user_properties`, `request_device_code`, `exchange_device_access_token`) + their impls; `oauth_client` field + `create_oauth_client`; `reconstruct` fns in persistence; `reset_initial_load`; `on_user_changed`; `AuthRedirectPayload`. **22 files, −1,754 LoC. 3-gate 0/0/0.** |
| firebase crate + token-refresh path | ✅ done | `62d8fbe0` | Deleted `crates/firebase/` (FetchAccessTokenResponse, FirebaseError, AccountInfo). Stripped `fetch_auth_tokens` + `fetch_access_token_via_proxy` + `UserAuthenticationError` from `server_api/auth.rs`; `Credentials::Firebase` arm simplified to return cached id_token (no refresh). Cascade: `ServerApiEvent::NeedsReauth` + `AuthManagerEvent::NeedsReauth` + `AuthManager::set_needs_reauth` + `AuthState::set_needs_reauth` all deleted (never constructed once firebase refresh gone). Dead subscribers cleaned in agent_sdk, connected_self_hosted_workers, remote_server/mod.rs. 14 files, −450 LoC. 3-gate 0/0/0. |
| Experiments (A/B) `app/src/server/experiments/` | ✅ done | `2a908b84` | Deleted 4 files (mod.rs, model.rs, convert.rs, model_tests.rs) + `mod experiments;` in server/mod.rs. Cascade: persistence/mod.rs (`experiments` field + `SaveExperiments` variant), sqlite.rs (load block + save arm + `save_experiments` fn), server_api.rs (`handle_experiments_fetched`), workspaces/user_workspaces.rs + gql_convert.rs (field + macro call), lib.rs (import + `new_from_cache` call + destructure var), 3 test files. −576 LoC. 3-gate 0/0/0. |
| AuthClient privacy-sync methods | ✅ done | `a0c02e86` | Removed from trait + impl: `create_anonymous_user`, `get_user_settings`, `set_is_telemetry_enabled`, `set_is_crash_reporting_enabled`, `set_is_cloud_conversation_storage_enabled`, `update_user_settings` + `SyncedUserSettings` struct. Deleted `PrivacySettings` `auth_state`/`auth_client` fields + server-sync wrappers (`fetch_or_update_settings`, `initialize_from_fetched_settings_or_update_settings`, `overwrite_local_settings_if_cloud_disabled`, `update_server_with_local_settings`). `create_anonymous_user` debug command removed from `global_actions.rs`. 3 files, −363 LoC. 3-gate 0/0/0. |
| AuthView / AuthOverrideWarningModal cloud-gate UI | ✅ done | `b9b1cf1c` | Delete 5 auth view files (auth_view_modal, auth_view_body, auth_view_shared_helpers, auth_override_warning_modal, auth_override_warning_body). Drop `AuthManagerEvent` enum + `attempt_login_gated_feature` + `anonymous_user_hit_drive_object_limit` from AuthManager (Event = ()). Remove `auth_override_warning_modal` + `require_login_modal` fields/builders/handlers/render guards from workspace/view.rs. Remove `blocked_for_anonymous_user` gate blocks + `LoginGatedFeature` From impls from drive, settings (ai_page, billing×2, main_page, teams_page), search, terminal (~18 sites). Simplify drive_helpers anonymous-limit helpers to return false. Drop dead count checks in update_manager create_{notebook,workflow,env_var_collection}. **21 files, −2,088 LoC. 3-gate 0/0/0.** |
| **Billing/teams/platform/referrals UI strip + AuthClient billing methods** | ✅ done | `ac8a95c3` | Delete billing_and_usage_page{,_v2,_tests}.rs, billing_and_usage/ dir (11 files), teams_page.rs, platform_page{,_tests}.rs, platform/ dir (3 files), referrals_page.rs, transfer_ownership_confirmation_modal.rs, tab_menu.rs, admin_actions.rs, cloud_action_confirmation_dialog.rs, tab_selector.rs, word_block_editor.rs, clickable_text_input.rs, update_manager_tests.rs, mod_tests.rs. Remove `get_conversation_usage_history` + `list_agent_identities` + `AgentIdentity`/`AgentIdentitiesResponse` from AuthClient trait + impl. Remove dead team CRUD methods from workspaces/update_manager (create_team/leave_team/rename_team + callbacks + TeamUpdateManagerEvent enum). Remove remove_team_objects from server/cloud_objects/update_manager. SettingsSection::{BillingAndUsage,Teams,Referrals,OzCloudAPIKeys} redirected to Account across all callers. `create_discount_badge` inlined into mod.rs (still needed by buy_credits_banner + enable_auto_reload_modal in terminal/). **48 files, −20,168 net LoC. Binary 885.1 → 875.7 MB (−9.30 MB). 3-gate 0/0/0.** 3 deferred warnings: `DisplayMode::Settings` + `new` in `ai/blocklist/usage/conversation_usage_view.rs`; `next_refresh_time_local`/`total_current_workspace_bonus_credits_remaining` in `ai/request_usage_model.rs` — AI territory, defer. |
| Dead auth/billing/teams residue sweep | ✅ done | `6689b0b1` | `login_failure_notification.rs` deleted; `blocked_for_anonymous_user` method gone; `AuthFlowInstructions` re-export dropped; pre-existing `--tests` compile error fixed (`AuthManager` import); `unthemed_window_border` + orphan imports gone from root_view; `BillingAndUsagePageAction::{OpenUrl,ContactSupport}` + `TeamsPageAction::{LeaveTeam,OpenWarpDrive,ContactSales}` + full cascade (`TeamsPageViewEvent::OpenWarpDrive` → `SettingsViewEvent::OpenWarpDrive` → workspace handler + `MainSettingsPageEvent::OpenWarpDrive` + its `#[allow(dead_code)]`) + `AdminActions::contact_sales` all deleted. 13 files, −182 LoC. Binary unchanged (linker-invisible). 3-gate 0/0/0. |
| Onboarding crate deleted, login-slide seam relocated | ✅ done | `007c18e8` | 4-item seam (`OnboardingIntention`, `AI_FEATURES`, `WARP_DRIVE_FEATURES`, `slides::layout`, `slides::slide_content`) moved into `app/src/auth/`: `layout.rs` and `slide_content.rs` copied verbatim as siblings to `login_slide.rs` and aliased; the 3 small items inlined at the top of `login_slide.rs`. `pub use` chain in `terminal/view/action.rs` re-pointed. −10,599 net LoC (10,636 deleted across 38 files − 37 inserted across the auth/ relocations). Binary: 914.0 → 913.6 MB (−454 KiB — most onboarding code paths were already dead-stripped, only symbol/debug residue shrinks). Leaves `app/Cargo.toml` feature flags `agent_onboarding` / `hoa_onboarding_flow` + `assets/onboarding/` bundles as orthogonal followup cleanup. |
| Onboarding feature flags + bundle entries scrub | ✅ done | `992dc39e` | `FeatureFlag::{AgentOnboarding, HOAOnboardingFlow}` enum variants + their 2 cfg-gated registration sites in `app/src/features.rs` + 4 `agent_onboarding` / `hoa_onboarding_flow` Cargo.toml feature decls + 5 stale `resources = ["assets/onboarding"]` bundle entries (asset dir already gone). −21 LoC, no rebuild needed. |
| FreeUserNoAi experiment + flag | ✅ done | `996760cc` | Zero external callers anywhere. Dropped `Experiment::FreeUserNoAi{Control,Experiment}` variants + 6 match arms across mod.rs (Display, FromString, TryFrom<Experiment>, set_active_experiment) + the `is_free_user_no_ai_experiment_active` fn + its orphaned `UserWorkspaces` / `CustomerType` imports + `FeatureFlag::FreeUserNoAi`. Catch-all `e => Err(...)` arm in TryFrom keeps dropped upstream variants safe. Graphql Experiment enum entries left alone (crate still blocked). −31 LoC. |
| crash_reporting + sentry + cocoa_sentry + heap_usage_tracking | ✅ done | `242147d3` | Full Sentry SDK strip across app + warp_core + warp_logging + ai. Drops 4 features + sentry/sentry-log/minidumper/crash-handler deps + `app/src/crash_reporting/` dir (4 files) + 22 cfg sites + Sentry framework download/link block in app/build.rs + 5 osx_frameworks bundle entries + `FeatureFlag::{CocoaSentry, CrashReporting, LogExpensiveFramesInSentry}`. 32 files, −2,615 LoC source + −780 lines Cargo.lock (transitive deps gone). Binary near-zero delta (features were OFF by default). `PrivacySettings.is_crash_reporting_enabled` kept — persisted UI setting, retires with broader privacy/auth pass. |
| Dead telemetry residue prune | ✅ done | `7803fedc` | `crates/ai/src/telemetry.rs` deleted (only contained the dead `CodebaseContextSyncType` enum, zero callers) + `mod telemetry;` in ai/lib.rs + 6 unused imports in events.rs (SharedSessionId, TimingDataPoint, AgentViewEntryOrigin, NotebookTelemetryAction, CommandSearchItemAction, SessionId). −16 LoC. Warnings 120→113 (−7). |
| Tier-D rebrand: `warp-oss` → `sublight` | ✅ done | `6d4b4a14` | App-surface rebrand only — internal `warp_*` library crates keep their names (honest upstream attribution). Cargo `[[bin]]` name + `default-run` + the bundle.bin.oss metadata (`identifier`, `name`) + the embedded macOS Info.plist in `app/src/bin/oss.rs` (CFBundle{DisplayName,Executable,Identifier,Name}, URL scheme, copyright) + `AppId::new` + log filename + WINDOW_TITLE + 5 toast titles in root_view.rs + macOS app menu label + the "About Warp" command description all switch to Sublight. Other channel bins (stable/preview/dev/local/integration) keep their dev.warp.* identifiers for now. Zero binary delta. |
| README rewrite + brand/ assets landed | ✅ done | `1f946e6f` | Replaces upstream Warp marketing README (Oz / build.warp.dev / Slack / CoC / sponsor banner) with a Sublight-focused README: what's stripped, what's kept, how to build (`cargo run --bin sublight --features gui`), licensing (AGPL §5 modified-Warp disclaimer), upstream attribution. Lands brand/ (wordmark + mark + icon SVGs + favicons + app icons). |
| Upstream Warp channel bins dropped | ✅ done | `b847ec27` | Five thin entry-point bins (stable/preview/dev/local/integration) plus their shared channel_config.rs were Warp release-pipeline scaffolding — Sublight doesn't ship through it. Deletes the 6 .rs files, the 5 `[[bin]]` decls, and the 4 dead `[package.metadata.bundle.bin.*]` sections. Also fixes the bundle.bin section name from the stale `warp-oss` to `sublight` and refreshes its copyright/description for Sublight. `Channel::{Stable, Preview, Dev, Local, Integration}` enum variants stay (referenced by ~30 autoupdate / appearance / auth_state / code_review / etc sites) — pruning them is a follow-up cascade. −352 net LoC. |
| LoginSlideView subtree deleted | ✅ done | `0afda117` | LoginSlideView (the legacy onboarding-era multi-step login slide) has zero external constructors anywhere — its only caller was `AuthOnboardingState::Onboarding` in root_view, which was removed in the onboarding flow strip. The actual live login UI is AuthView (auth_view_modal.rs) and is unaffected. Deletes login_slide.rs + login_slide_layout.rs + login_slide_content.rs (the latter two relocated into auth/ at `007c18e8` to preserve a seam that turned out to lead nowhere) + mod decls + init call + dead `pub use OnboardingIntention` chain. −1812 LoC. Warnings 113 → 102 (−11). **Lesson: when preserving a 'seam', verify the view it feeds still has live constructors — otherwise the preservation is wasted work that needs another delete pass.** |
| Channel::Oss string rebrand | ✅ done | `68321755` | `cli_command_name`, `Display`, `url_scheme`, and `ChannelState::init()` AppId switched from `warp-oss` / `warposs` / `("dev","warp","WarpOss")` to `sublight` / `("local","sublight","Sublight")`. Closes a gap left by the binary rename at `6d4b4a14`. |
| Dead FeatureFlag variant sweep (17 total) | ✅ done | `de2b7e0c` + `8d50a54e` | Variants with zero callers anywhere: WelcomeTips, ThinStrokes, WelcomeBlock (`de2b7e0c`); then WithSandboxTelemetry, CloudObjects, FetchChannelVersionsFromWarpServer, ContextChips, FetchGenericStringObjects, IntegratedGPU, AgentPredict, LazySceneBuilding, AIBlockOverflowMenu, AIGeneratedOnboardingSuggestions, AIMemories, GetStartedTab, MarkdownImages, CloudModeHostSelector (`8d50a54e`). The MarkdownImages flag had zero readers — removing the variant doesn't change image rendering behavior (BlocklistMarkdownImages handles the agent-block path and is kept). −34 LoC. |
| Dead-telemetry residue sweep across 5 files | ✅ done | `40808ba5` + `dc6613d2` + `88c7dc81` + `d822c832` | Pruned dead event-payload types in `ai/blocklist/action_model/execute/request_file_edits/telemetry.rs` (9 dead structs, file 133 → 18 LoC), `ai/blocklist/telemetry.rs` (10 dead types, file 263 → 68 LoC), `ai/agent_management/telemetry.rs` (3 dead enums + SetupGuideStep::VisitOz variant, file 48 → 15 LoC), `ai/ambient_agents/telemetry.rs` (CloudModeEntryPoint dropped; HandoffEntryPoint kept), `code/lsp_telemetry.rs` (whole file deleted — both enums had no callers), `code_review/telemetry_event.rs` (GitButtonKind, GitDialogStatus::Cancelled, AddToContextOrigin). −424 LoC total; warnings 102 → 68. |
| Further dead-helper sweep | ✅ done | `0e18ea60` + `5ae114fb` + `c9ed6abf` | Drops FindOption / CloseTarget / OpenedWarpAISource enums in events.rs along with their sole constructors in quit_warning + ai_assistant (`0e18ea60`); the IS_CRASH_RECOVERY_PROCESS_RUNNING tracker (its reader died with the crash_reporting module) + env_presence/host_presence orchestration predicates (`5ae114fb`); resolve_orchestration_harness_label + AuthManager::upgrade_url + ServerApi::user_id + RequestedEditResolution::Accept variant (`c9ed6abf`). Combined −112 LoC; warnings 68 → 57. |
| PasteAuthTokenModal + 2 orphan helpers | ✅ done | `47590380` | Whole 376-LoC `auth/paste_auth_token_modal.rs` subsystem deleted — PasteAuthTokenModalView had zero constructors anywhere; the field on root_view that was supposed to hold it was always None. Plus MEANINGFUL_EDIT_THRESHOLD (notebooks/notebook.rs) and AIBlockResponseRating::name() helper. −470 LoC. |
| AIAgentInput + NotebookTelemetryMetadata cascade | ✅ done | `e48fc80e` | AIAgentInput enum + its 18-arm From<FullAIAgentInput> impl gone; NotebookTelemetryMetadata struct + its `new` / `with_markdown_table_count` impl + the `telemetry_metadata` / `open_telemetry_metadata` methods on Notebook (in notebooks/notebook.rs and notebooks/file/mod.rs); CodeContextDestination::AgentInput variant. −152 LoC. |
| CpuUsageStats / MemoryUsageStats / BlockMemoryUsageStats payloads | ✅ done (initial pass) | `85804c6d` | Dropped the 3 events.rs payload structs + their `From<Local> for telemetry::Payload` impls in system/info.rs. The local structs in system/info.rs were temporarily kept with `#[allow(dead_code)]` because info_tests.rs exercised them. −76 LoC. |
| Tips / VerticalTabs / Callout / CallOut color helper sweep | ✅ done | `35a175e1` + `328257e5` | WelcomeTipFeature enum + impl (`35a175e1`); VerticalTabsDisplayOption enum + VerticalTabsChipEntrypoint::serialized helper (`35a175e1`); CalloutArrowPosition::{Start,End} variants + their 4 match arms in Up / Left arrow layouts (`35a175e1`); callout_title_color + callout_body_color helpers (`328257e5`). −141 LoC. |
| Dead malformed-terminal-line module + 4 unused counters | ✅ done | `016c13f2` | The CodeDiffView apply-diffs flow tracked edited_file_count / correction_count / edited_correction_count / unedited_correction_count counters that fed deleted telemetry payloads, plus a `if correction_count > 0 {}` empty branch. Dropping the counters orphaned `has_malformed_terminal_correction_signal`'s sole call site → whole `inline_action/malformed_line_heuristics.rs` module (12 fns) + its 200-LoC test file deleted. Also clears is_cross_repo / did_auto_trigger_request / num_blocks_reverted counter variables. −330 LoC. Warnings: 113 → 23 (this commit's most concentrated drop). |
| 0-warning push — 11 helpers + step/rule + repo_is_local + TabTelemetryAction + final 5 | ✅ done | `3b5bd85f` → `15a7c61e` | A run of small targeted prunes: is_ambient method, PassiveCodeDiffRequestStarted variant payload, refresh_public_models + refresh_available_models (only alive under `#[cfg(feature = "agent_mode_evals")]` which isn't built), build_plan_yearly_price_cents, active_query_filter, interrupt_block, to_string helper, to_telemetry_mode (40-LoC match block), workflow_space method, LocalToCloudHandoffIntent::entry_point, entrypoint + cli_agent fields on RightPanelUpdateParams, bootstrap_start + background_executor fields on TerminalView, reason field on TerminationType::Premature, step/rule fields (cloud_setup_guide + suggested_rule_modal events + handlers), agent_management/telemetry.rs module deletion (SetupGuideStep retired with it), 3 dead repo_is_local methods (code_review_view, comment_list_view, find_model), TabTelemetryAction enum, NotebookTelemetryMetadata cascade through notebooks/, plus the literal final 5 warnings (BlocklistAIHistoryModel import, prompt_suggestion_id / code_exchange_id locals, Itertools test import). Warnings 16 → 0. |
| Replace 3 `#[allow(dead_code)]` cheats with real deletions | ✅ done | `eb1785e0` + `f6789b13` | The 0-warning checkpoint at `15a7c61e` had 3 cosmetic suppressions (CpuUsageStats / MemoryUsageStats structs + CodeDiffView::edit_format_kind field). Replaced each with the real cascade-delete: the entire ResourceUsageReporter resource-usage pipeline gutted (struct + 5 methods + 3 timing constants + Default + CpuUsageStats / MemoryUsageStats / BlockMemoryStats / Sample / StatsBuffer / REPORT_* constants + cpu_usage method + 2 SystemInfo fields + their refresh-loop call sites + SystemInfo::handle_block_created and its caller in terminal/view.rs + info_tests.rs + 10 orphan imports) at `eb1785e0` (−427 LoC); RequestFileEditsFormatKind enum + telemetry.rs file + 4 fn signatures (CodeDiffView::{new, new_passive, build}, on_maa_code_diff_generated) + the field + classify_edit_format helper + block.rs match arm + NewCodeDiffSuggestion event field + terminal/view.rs unpack + 8 re-exports across crates at `f6789b13` (−73 LoC). Final 3-gate: 0 errors, 0 warnings on all 3, no `#[allow(dead_code)]` cheats added in this session. |

| Autoupdate pipeline strip | ✅ done | `e2aeb960` | Entire `app/src/autoupdate/` dir gone (8 files: mod + changelog + channel_versions + linux + mac + windows + 2 test files). `AutoupdateState`/`AutoupdateStage`/`AutoupdateStateEvent`/`RequestType`/`UpdateReady`/`DownloadReady`/`ReadyForRelaunch`/`RelaunchModel` all gone. Consumers stripped: `WorkspaceAction::{ApplyUpdate,DownloadNewVersion,CheckForUpdate,AutoupdateFailureLink}` + handlers + bindings; `MainPageAction::{Relaunch,DownloadUpdate,CheckForUpdate}` + `MainSettingsPageEvent::CheckForUpdate` + `SettingsViewEvent::CheckForUpdate`; `WorkspaceBanner::{VersionDeprecated,UnableToUpdateToNewVersion,UnableToLaunchNewVersion}` + dismissal state; `BindingGroup::AutoUpdate`; `FeatureFlag::{Autoupdate,AutoupdateUIRevamp}`; `Workspace`/`WorkspaceArgs`/`RootView` `server_time` plumbing; `ChangelogModel` autoupdate dep stubbed to `Ok(None)`; `ModelEvent::FinishUpdate` collapsed to no-op. 26 files, −4382 net LoC. Binary 913.3 → 911.7 MB (−1.47 MB — consumer chain RootView→server_time→banner was live). 3-gate 0/0/0. |
| Channel enum cascade | ✅ done | `d58d199e` | `Channel::{Stable,Preview,Dev,Local,Integration}` variants all dropped — `Channel::Oss` is sole variant. `is_dogfood`/`allows_server_url_overrides`/`cli_command_name`/`Display` impls collapse to constants. `ChannelState` branches (`enable_debug_features`/`url_scheme`/`base_warp_config_dir_name`/`secure_state_dir`) collapse to Oss-path values. Cascade: remote_server/setup.rs (6 fns inlined, `pinned_version` deleted), http_client (integration-only env-var path gone), isolation_platform (test bypass gone), warp_completer (per-channel CLI registration → single sublight bin), app layer 22 sites (preview URL params, integration test bindings, channel-discrimination branches). `test_allow_user_overrides` deleted. 32 files, −435 LoC. Binary 911.7 → 911.6 MB (−104 KiB). 3-gate 0/0/0. |

**Two distinct "welcome" surfaces — do not confuse:**
1. **`welcome_palette` pane** — a tab's content ("Code, build, or search for anything…"). ✅ REMOVED (`59562bd7`).
2. **First-run onboarding/login gate** — full-screen "Welcome to Warp / Get started / Log in", rendered at `root_view.rs` via `auth_onboarding_state` → `AuthOnboardingState::Auth` when logged-out. This is what shows on launch and currently **masks** surface #1. Belongs to the onboarding/auth removal (🔄 NEXT). A bypass exists (`SkipFirebaseAnonymousUser` path → `Terminal`) but we're doing the proper onboarding removal instead.

**Verify-welcome reminder**: after onboarding/login gate is removed, relaunch and confirm (a) first launch lands on a terminal, (b) `Ctrl+T` opens a terminal, (c) no welcome palette anywhere.

### Onboarding removal sub-plan (in progress)

Scope: remove the onboarding experience + crate. **KEEP login** (`AuthView`, `AuthOnboardingState::Auth`) — separate later pass. Crate `onboarding` is app-only-dep → deletable. Login seam is small/clean: `login_slide.rs` uses `slides::layout` (380) + `slides::slide_content` (71) + `OnboardingIntention` + `AI_FEATURES`/`WARP_DRIVE_FEATURES`, all self-contained (~470 LoC, no intra-crate deps).

- **Commit 1 — remove flow** (big): delete app onboarding modules (`ai/onboarding.rs`, `settings/onboarding.rs`+tests, `workspace/view/onboarding.rs` `OnboardingTutorial`, `workspace/hoa_onboarding/`, `terminal/view/block_onboarding/`, `experiments/block_onboarding_layer.rs`, orphaned `coding_entrypoints/`); remove `lib.rs onboarding::init`, `workspace hoa_onboarding::init`; `root_view` `AuthOnboardingState::Onboarding` + `create_agent_onboarding_view` + the ~283-LoC `AgentOnboardingEvent` handler; `workspace/view` `check_and_trigger_onboarding`/`trigger_*`; `terminal/view` `onboarding_callout_view` field + handler + `OnboardingFlow` action; feature flags `agent_onboarding`/`hoa_onboarding_flow` + assets. After: app references crate only via login_slide + `terminal/view/action` re-export.
- **Commit 2 — relocate login bits**: move `layout`/`slide_content`/`OnboardingIntention`/`AI_FEATURES`/`WARP_DRIVE_FEATURES` into `app/src/auth/`; re-point `login_slide.rs` + drop `onboarding` from `app/Cargo.toml`. Login works, sourced from auth/.
- **Commit 3 — delete crate**: remove `crates/onboarding/` + workspace `Cargo.toml` member.

Green (`cargo check -p warp`) at every commit.

---

## 1. Telemetry — STRIP COMPLETE ✅

All telemetry surfaces removed across ~10 commits. Total: ~8918 LoC across step 1 (no-op macros, `4b2d993f`) → step 2 (kill live send path, `a8ae755c`) → step 3 (delete central `TelemetryEvent` enum, `a9b89f9b`, −6966 LoC) → step a (16 satellite enums, `e466235a`/`2eebd0fe`/`8921cfcb`) → step e (warning sweep 795→142, `bcd1fc6a`) → step b (warp_core trait machinery, `081a425a`) → step c (c1-c8: dispatch infra, `7c2fe36b`..`3dadef72`) → cleanup-1 (819 macro call-sites + 5 macro defs, `a052b2a3`, −5802 LoC across 193 files) → cleanup-2 (43 dead events.rs payload structs, `c7319c0a`).

**State**: zero telemetry queue / dispatcher / sender / payload struct / macro / trait / event type anywhere in the workspace. 42 surviving payload items in events.rs are plain data carriers for non-telemetry features (PaletteSource, CLIAgentType, AIAgentInput, etc.). GUI binary builds: `cargo build --bin warp-oss --features gui` = 0 errors, 914 MB.

**Still to scrub** (orthogonal, separate cuts): `crash_reporting`, `profiling`, the 4 analytics feature flags (`GlobalAIAnalyticsCollection`, `AgentModeAnalytics`, `RecordAppActiveEvents`, `WithSandboxTelemetry`) — none of these are telemetry-dispatcher dependent now; they're just inert flag definitions / optional heap-uploader scaffolding. Delete when convenient.

---

## 2. Warp AI — REMOVE (Warp's proprietary LLM agent/assistant)

| Finding | Location | Size | Difficulty | Why |
|---|---|---|---|---|
| ~~`ai_assistant` panel~~ | ~~`app/src/ai_assistant/`~~ | ~~10 files, 3.6k LoC~~ | **EASY** | ✅ **DONE** `782368f5` — panel + `warp_ai.rs` command search + all callers deleted. `WarpAiExecutionContext` → `ai/execution_context.rs`; `AskAIType` → `terminal/view.rs`. −4568 LoC, 35 files. 3-gate 0/0/0. |
| `ai` engine crate | `crates/ai/` | 75 files, 25k LoC | **MEDIUM** | Fairly self-contained, but 628 app callers — delete only after callers refactored. |
| ~~Execution profiles + model selector~~ | ~~`app/src/ai/execution_profiles/`, `terminal/profile_model_selector.rs`~~ | ~~9 files~~ | **MEDIUM** | ✅ **DONE** `e3e2c025` + `85902edb` — profiles.rs (1,416 LoC) + model_menu_items.rs + tests + inline selector UI + profile_model_selector.rs (2,342→82 stub) deleted. `AIExecutionProfilesModel` stub re-registered as singleton. Permission types (`ActionPermission` etc.) kept in stub for `workspaces/`. −5,534 net LoC. Binary 846.9→844.2 MB. 3-gate 0/0/0. App launches cleanly. |
| `context_chips` (terminal prompt chips) | `app/src/context_chips/` | 23 files, 11.7k LoC | **KEEP** | Core terminal prompt rendering: git branch, directory, virtualenv, SSH, k8s chips. NOT a cloud/AI surface. Do NOT remove. |
| ~~AI settings pages~~ | ~~`settings_view/{ai_page,execution_profile_view}.rs`~~ | ~~~9.4k LoC~~ | **MEDIUM** | ✅ **DONE** `111a175e` — ai_page.rs (7824) + editor/ (1992) + modals + warp_drive_page deleted. −13,934 LoC. Binary −25.1 MB. |
| `app/src/ai/agent` (LLM exec core) | `app/src/ai/agent/` | 32 files, 22.5k LoC | **HARD** | Warp agent driver/harness/todos/SDK. |
| `app/src/ai/blocklist` (AI block render) | `app/src/ai/blocklist/` | 179 files, 102k LoC | **HARD** | `agent_view/` deleted (`94c6be3f`). model layers (controller/history/context/input/action) remain; see Phase F/G below. |
| Conversation/history models | `app/src/ai/blocklist/history_model.rs`, `agent_conversations_model.rs` | ~15k LoC | **HARD** | AI chat state, conversation IDs; on-disk. |
| Render-path AI coupling | `terminal/view.rs` (203 refs), `input.rs` (151), `pane_group/mod.rs` (95), `workspace/view.rs` (105) | — | **HARD** | Core spider files — excise AI branches, keep render. Do LAST. |

**Total**: `app/src/ai` 453 files / 217k LoC + `crates/ai` 75 files / 25k LoC. Hotspot: `terminal/view.rs`.

---

## 3. Warp proprietary — needs Warp's backend (login/cloud/sharing/teams/billing)

| Finding | Location | Size | Difficulty | Why |
|---|---|---|---|---|
| **firebase crate** | `crates/firebase/` | 1 file, 145 LoC | **EASY** | ✅ **DONE** `62d8fbe0` — crate deleted; token-refresh path stripped. |
| ~~Experiments (A/B)~~ | ~~`app/src/server/experiments/`~~ | ~~4 files, 521 LoC~~ | **EASY** | ✅ **DONE** `2a908b84` — 4 files deleted; cascade through persistence, sqlite, server_api, workspaces, lib.rs. −576 LoC. |
| Cloud network crates | `graphql`, `warp_server_client`, `warp_graphql_schema`, `websocket`, `managed_secrets`, `warp_web_event_bus` | ~6 crates | **EASY*** | Pure API layers — *but `websocket`/`graphql` pulled by `warp_core`/`ai`, so blocked until those callers go. `warp_web_event_bus` = wasm-only, removable now. |
| Cloud preferences syncer | `app/src/settings/cloud_preferences.rs` | 1 file, ~250 LoC | **MEDIUM** | ✅ DONE (`7fb92e80` Inc A + Inc B). Toggle + CloudPreference model gone; WarpDrivePrivacySettings local store KEPT; inert protocol enums KEPT. |
| Teams / billing / API keys | `settings_view/{teams_page,billing_and_usage*}`, `app/src/billing/` | ~4.4k LoC | **MEDIUM** | Feature-gated pages. |
| Shared sessions | `app/src/terminal/shared_session/` | 39 files, 5.1k LoC | **MEDIUM** | Session relay UI/logic. |
| Auth / login UI | `app/src/auth/` | 22 files, 7.8k LoC | **MEDIUM** | Firebase token + login UI; 174 fan-in. **Partial:** login gate + auth gate UI + AuthManager network calls + firebase crate + privacy-sync methods + AuthView/AuthOverrideWarningModal cloud-gate surfaces all done. Remaining: `AuthClient` API-key / billing / conversation-usage methods (live UI); `AuthView` / `AuthOverrideWarningModal` types + atom UI gone. |
| Cloud sync / Warp Drive | `app/src/drive/` + `cloud_preferences*` | 47 files, 22.6k LoC | **MEDIUM→HARD** | Cloud object indexing/sharing; 100+ fan-in. |
| Code review (cloud/remote) | `app/src/code_review/` | 40 files, 23.5k LoC | **MEDIUM→HARD** | Remote review/indexing; terminal-coupled. |
| Oz / ambient agents / cloud mode / orchestration / handoff | `app/src/ai/ambient_agents/` + blocklist orchestration | ~2k+ LoC | **MEDIUM** | Server-side agent runs. |
| **`server/` cloud API module** | `app/src/server/` | 55 files, 40k LoC | **HARD** | GraphQL/sync/cloud-objects/experiments; **454-file fan-in**. The backend spine. |

**firebase note**: removing it forces touching `server/server_api/auth.rs`, which is inside the HARD `server/` module — so firebase deletes cleanly *as part of* removing the auth/server layer, not standalone.

---

## 4. Useless features — onboarding / tutorials / hints / modals / nags

| Finding | Location | Size | Difficulty | Why |
|---|---|---|---|---|
| Build-plan migration modal | `workspace/view/build_plan_migration_modal.rs` | 870 LoC | **EASY** | One-time modal; 6 refs. |
| One-time modal infra | `workspace/one_time_modal_model.rs` | 508 LoC | **EASY** | Base for migration/launch modals; 11 refs. |
| Quit warning | `app/src/quit_warning/` | 508 LoC | **EASY** | Feature-gated; 8 refs. |
| Welcome/landing screen | `app/src/search/welcome_palette/` (858) + `pane_group/pane/{welcome_view,welcome_pane}.rs` | ~1.5k LoC | **MEDIUM** | New-tab landing surface, NOT throwaway. Remove → new tab defaults to `LeafContents::Terminal`; drop `Welcome` variant + sqlite persistence. Its actions (terminal/workflows/search/add-repo) all exist in command palette; strip AI "new conversation" + telemetry branches. |
| Changelog section | `resource_center/section_views/changelog_section.rs` | ~100 LoC | **EASY** | Removable subsection. |
| Launch/announcement modals | `workspace/view/launch_modal{,_oz,_orchestration}.rs` | 1.9k LoC | **MEDIUM** | `workspace/view.rs` 56 refs; 3 flavors. |
| ~~Resource center~~ | ~~`app/src/resource_center/`~~ | ~~35 files, 2.2k LoC~~ | **MEDIUM** | ✅ **DONE** `4f942dbf` — ResourceCenterView panel deleted; TipsCompleted/Tip/KeybindingsView kept (load-bearing for terminal tips/settings). −2,922 LoC, −1.04 MB. |
| Get-started landing tab | `pane_group/pane/get_started*.rs` | 484 LoC | **MEDIUM** | Sibling landing surface (`get_started_tab` feature). Same as welcome screen — remove → new tab defaults to terminal pane. Feature-gated; 21 refs. |
| Referrals | `referral_theme_status.rs` + `referrals_page.rs` | ~400 LoC | **MEDIUM** | Settings subsection; auth-coupled. |
| ~~Bonus grant notification~~ | ~~`workspace/bonus_grant_notification_model.rs`~~ | ~~132 LoC~~ | **MEDIUM** | ✅ **DONE** `98278b0f` |
| ~~Buy credits banner + auto-reload modal~~ | ~~`terminal/buy_credits_banner.rs` + `terminal/enable_auto_reload_modal.rs`~~ | ~~1,362 LoC~~ | **MEDIUM** | ✅ **DONE** `4b9da44d` — Warp billing UI; `OpenAutoReloadModal` event chain excised from 9 files. −1.56 MB. Dead methods (`compute_buy_addon_credits_banner_display_state` / `dismiss/enable_buy_credits_banner` / `BuyCreditsBannerDisplayState`) cleaned up in `60d136cf`. |
| Onboarding crate | `crates/onboarding/` | 36 files, 11.5k LoC | **MEDIUM** | ⚠ Depended on by `ai` crate — partly blocked by AI removal. Feature-gated. |
| Onboarding UI (block + HOA) | `terminal/view/block_onboarding/` (1.6k) + `workspace/hoa_onboarding/` (1.1k) | ~2.7k LoC | **MEDIUM** | ~104 refs in `terminal/view.rs`, modular. |
| Agent tips | `app/src/ai/agent_tips.rs` | 673 LoC | **MEDIUM** | Feature-gated; AI-coupled. |
| Warpify footer/banner | `app/src/terminal/warpify/` | 53 files, 1.8k LoC | ✅ DONE (S60, 10 commits `15a0f0ed`→`e6ef17e8`) | Feature fully removed (disable→orchestration→block-banner→modules→actions→ssh→render), GUI-verified. ~29 SSH-transport/ANSI-coupled dead-code warnings remain → `ai-strip-session-60-warpify.md`. Kept generic subshell flag + draw_flag_pole. |
| Tips/hints sidebar | `app/src/tips/` | 47 files, 726 LoC | **HARD** | 138 refs in `terminal/view.rs`; cascades to AI panel. |
| Notebooks (cloud) | `app/src/notebooks/` | 30 files, 22.4k LoC | **HARD** | 226 fan-in; drive/search/AI-coupled. |

---

## Recommended removal order (green at each step)

1. **Telemetry no-op + crash-reporting/profiling** (EASY, mandated). No-op macros → delete crash_reporting/profiling/analytics flags. Big risk-reduction early.
2. **firebase + experiments + wasm-only crates** (`warp_web_event_bus`) — small clean deletions.
3. **Useless EASY items** — quit_warning, welcome_palette, one-time/migration/launch modals, changelog, resource_center, referrals, bonus-grant. Visible, low-coupling wins.
4. **Teams/billing/shared-sessions/cloud-prefs/auth UI** (MEDIUM) — feature-gated proprietary panels.
5. **Telemetry infra + call-site cleanup** — delete event enum, collector, dispatch, warpui_core/warp_core telemetry; remove dead call-sites.
6. **AI mid-tier** — ai_assistant panel, execution_profiles, context_chips, AI settings pages, agent_tips, onboarding crate.
7. **Drive / code_review / notebooks / server cloud API** (HARD) — the backend spine + large cloud subsystems.
8. **Core spider files LAST** — `terminal/view.rs`, `terminal/input.rs`, `pane_group/mod.rs`, `workspace/view.rs`, `root_view.rs`: excise AI/cloud/warpify/tips branches, **preserve render**.
9. **Delete drained engine crates** — `ai`, `graphql`, `warp_server_client`, `websocket`, `managed_secrets`, `onboarding` once fan-in is zero.
10. **Scrub** — dead feature flags, dormant config, grep for phone-home (warp.dev/firebase/rudderstack). Build `sublight --features gui`, launch, verify render.

---

## Resume notes (post-compaction continuation)

**Where**: worktree `~/Documents/git/warp-upstream`, branch `surgical-strip` off upstream `0b737e22` (green baseline). `strip-cloud` (old delete-foundation-first attempt) is abandoned — ignore its `AGENTS.md`; this `plan.md` + `docs/superpowers/specs/2026-05-27-surgical-cloud-strip-design.md` are the living docs.

**Verify green** (3-gate matrix — background in parallel, slow):
- `cargo check -p warp` (default)
- `cargo check -p warp --tests`
- `cargo check -p warp --features local_fs,gui` (combined — features compose)

All 3 must be 0 errors before each commit.

**Build + launch the GUI**: `cargo run --bin sublight --features gui` (NOT `./script/bootstrap` — Debian/apt-only; CachyOS deps already present). First `--features gui` build is long. Confirmed: app launches straight to a terminal (no welcome/onboarding/login). GUI binary build verified post-telemetry-strip (`c7319c0a` as `warp-oss`, then again post-rebrand as `sublight`).

**Done + verified**: welcome panes (`59562bd7`), onboarding flow (`3e87396f`), **telemetry strip COMPLETE** (`c7319c0a`). Login KEPT. Brand assets done in `brand/` (untracked).

### Drive-UI / sharing pass — ✅ COMPLETE

- **Cloud-import UI REMOVED (`af644a57`).** `drive/import/` dir (~2k LoC) + trigger chain (DriveIndex/Panel `OpenImportModal` action+event+2 menu items+IMPORT_LABEL, workspace `import_modal`+`is_import_modal_open`, `WorkspaceAction::{ImportToPersonalDrive,ImportToTeamDrive}`+bindings).
- **AI-conversation share UI REMOVED (`fd66258a`, 3-gate 0/0/0):** terminal `CopyConversationShareLink`/`OpenConversationShareDialog` TerminalActions + context_menu 2 share items + pane_impl `agent_view_shareable_object` + agent-pane share-button + conversation_list+item `OpenShareDialog`/`sharing_dialog`/per-item overlay.
- **SharingDialog graph + permission-CRUD payoff REMOVED (`d93aac09`, 52 files, −4,767 net LoC, 3-gate 0/0/0).** The originally-staged 1b–5 boundaries were NOT self-contained — `toggle_share_dialog`/`open_object_sharing_settings` was a **shared sink** fed by drive-index + notebook/workflow invitee + pane-header + inheritance, and every feeder existed only to open a dialog that was itself being deleted. So the real unit was the whole `SharingDialog` invocation graph, collapsed sink-first in one WIP commit:
  - Deleted `drive/sharing/dialog/` (SharingDialog view + inheritance) + `drive/sharing/` module (`ShareableObject`, `SubjectExt`/`UserKindExt`/`TeamKindExt`, qr_code, style). **`ContentEditability` relocated → `cloud_object/model/view.rs`**; `SharingAccessLevel`/`Subject`/`TeamKind`/`UserKind`/`LinkSharingSubjectType` consumers re-pointed to `warp_server_client::drive::sharing`.
  - drive-index: `ToggleShareDialog` action + `sharing_dialog`/`share_dialog_open_for_object` fields + handler + render branch + 2 Share menu items + `toggle_share_dialog`; `share_dialog_open` param removed from `WarpDriveRow::new`/`new_from_cloud_object`.
  - pane-header subsystem: `header/sharing.rs` (`SharedPaneContent`) + `render_sharing_controls`/`sharing_controls()` hook + `OpenOverlay::SharingDialog` + `PaneAction::ShareContents` (→ `PaneAction` uninhabited) + `PaneConfigurationEvent::{ShareableObjectChanged,ToggleSharingDialog,OpenSharingQrCode}` + emit methods + `set_shareable_object` feeders (ai_document/env_var/notebook/workflow) + `HeaderRenderContext` lifetime.
  - event chains: `OpenDriveObjectShareDialog` (notebook/workflow/pane_group/workspace) + `WorkspaceAction::OpenObjectSharingSettings` + `open_object_sharing_settings` wrappers; dead `SharingDialogSource` telemetry enum.
  - **PAYOFF:** 10 dead `UpdateManager` permission/guest methods + `ObjectOperation::UpdatePermissions` + `save_permissions_update`; `ObjectClient` permission methods + `ServerApi` impls + `GuestIdentifier` + 5 GraphQL mutation imports + `ObjectPermission{UpdateResult,UpdateData}`. Dialog-transitive `WordBlockEditor` API pruned (`with_layout`/`with_styles`/`Horizontal`/`Navigate` payload).
  - **KEPT (still UI-reachable):** `ObjectClient` trash/untrash/delete/move/leave/edit-access + their `UpdateManager` callers + `server_api/object.rs` CRUD for those; `SyncQueue`.
  - **Lesson — staged file:line recipes for a woven feature are a guess until you find the shared sink.** Sink-first collapse (delete central view/module, let `cargo check --features gui` enumerate feeders upward) beat the bottom-up stage plan. Uninhabited enum = honest "no actions" when `add_typed_action_view`/`TypedActionView` still needs the assoc type. `recast` for the mechanical `HeaderRenderContext<'_>` lifetime sweep (21 sites/18 files); python line-range deletes for big contiguous clusters; the dead-code/unused-import warnings drove the payoff worklist precisely.

### Warp Drive server-sync amputation — ✅ COMPLETE (`96207e43`, keep sqlite store, cut server leg)

**Landed (`96207e43`, 22 files, −13,140 net LoC, binary 899.4 → 896.9 MB, 3-gate 0/0):** Phase 0 behavioral amputation + deleted the inbound sync-merge cluster (`on_changed_objects_fetched` + handlers) + poll engine + `cloud_preferences_syncer` (settings cloud-sync) + server-sync test suites + dead test infra + MCP-gallery server-push path. Settings + Drive content stay local (sqlite + TOML). GUI smoke-tested (startup clean). **DEFERRED to the drive-UI/sharing pass (still KEPT, UI-reachable, NOT dead):** the per-object server-action methods (`trash`/`untrash`/`delete`/`move`/`leave`/guests/permissions/edit-access), `ObjectClient` trait + `impl for ServerApi` + `server_api/object.rs` CRUD, `SyncQueue` (`enqueue` still called by mutation methods; `dequeue` gated-off). These deliver the *next* Drive binary shrink once `drive/sharing/` + `drive/import/` UI is removed. Working notes below.

**Decision (2026-05-29, user-chosen)**: keep `cloud_object`'s sqlite store (= session-restore backend), amputate only the server-sync half. Drive content (workflows / env-vars / notebooks / MCP / AI facts / folders) stays **local-only** — no upload, no cross-device, no share links. App settings (themes/keybindings) were never Drive-backed.

**Architecture mapped (read-only, verified by grep/Read):**
- `local_fs` is NOT in `default`; `gui = ["voice_input"]` doesn't pull it → shipped build persists Drive objects via `cloud_object` = **sqlite + server**, not local files.
- **Read/populate path (KEEP):** `sqlite → PersistedData.cloud_objects → CloudModel::new(persistence_writer.sender(), cloud_objects, …)` at `lib.rs:1570`. CloudModel is sqlite-authoritative at startup — server bulk-load only merges on top. **Amputating server does NOT empty the drive** (advisor linchpin check — passed).
- **Write path (KEEP):** `CloudModel`/`UpdateManager.save_to_db` (`update_manager.rs:273`) → `model_event_sender` → sqlite.
- **Server leg (CUT):** `UpdateManager.object_client: Arc<dyn ObjectClient>` (`update_manager.rs:209`) + `SyncQueue` (outbound push, `lib.rs:1593`) + `Listener` (`server/cloud_objects/listener.rs`) + polling + `received_message_from_server` (RTC inbound) + has_initial_load gating + guests/sharing/permissions methods.
- **Seam:** `trait ObjectClient` (`server_api/object.rs:170`); real impl `impl ObjectClient for ServerApi` (`:336`); constructed at `lib.rs:1596/1679/1745` via `get_cloud_objects_client()`. `FakeObjectClient` exists but PANICS (`unimplemented!`) on mutations — NOT a usable no-op.
- **Callers of UpdateManager: 62** (4847-LoC hub). Split: **48 general keep-features** (workflows/env_vars/notebooks/drive panel+index+import/workspaces/settings_view) + **14 AI-panel** (facts/ambient/conversations) → AI methods stay no-op-server now, die with the AI strip.

**Strategy = collapse at the client boundary (Noop), preserve the public mutation API → 62 callers unchanged, zero throwaway plumbing:**
- **Phase 0 (PROVE, reversible):** add `NoopObjectClient` (returns success, no I/O, no panics) as the only `get_cloud_objects_client()` return; neuter `Listener` mount + `SyncQueue` push + polling startup. Build `--features gui`, smoke-test: create workflow/env-var/notebook → persists → restart → restored; no network. Proves the architecture before deleting anything.
- **Phase 1 (delete server I/O):** delete real `impl ObjectClient for ServerApi` + `server_api/object.rs` CRUD + `listener.rs` + polling + `received_message_from_server`/bulk-load + UpdateManager guests/sharing/permissions methods (pure-server). Keep every mutation method's `save_to_db`.
- **Phase 2 (simplify):** drop `object_client` field + `SyncQueue` + `ServerId` reconciliation in `persistence.rs` (objects stay `ClientId` forever — fine local-only); collapse `has_initial_load` gating (note: `auth/mod.rs`+`auth_manager.rs` login→initial-load trigger becomes no-op — expected, not a regression).
- **Phase 3 (cloud UI, separate sub-strips):** `drive/sharing/` (share dialog + QR) + `drive/import/` (cloud import) + settings_view teams/billing pages.
- **Defer to AI strip:** 14 AI-method callers + AI-specific UpdateManager methods.

**Phase 0 DONE + smoke-tested** (`wip-drive-server-amputation` @ `fc095262`): 4 edits (signal `has_initial_load` in `UpdateManager::new`; `start_polling`/`refresh_updated_objects`→no-op; `Listener::start_listener`→no-op; `dequeue` stays gated-off). Compiles `--features gui` 0 errors / 5 dead-code warns. GUI launches, terminal reached, no deadlock/crash (validates the signal). Drive-content load code-proven (`CloudModel::new` from sqlite @ `lib.rs:1571`, panel renders all, view.rs:344 is a sort-gate not a visibility filter).

**⚠️ CORRECTION (2026-05-29) — settings-sync mis-stated to user.** I told the user "app settings are local TOML, not Drive/server-synced." WRONG: `cloud_preferences_syncer` mirrors local prefs to the server as `CloudPreferenceModel` (= `GenericStringModel<Preference>`, a cloud object pushed via the same SyncQueue/object_client path). BUT local TOML persistence is owned by `settings/init.rs` + `settings/mod.rs` (NOT the syncer) → with server amputated, settings still persist locally; only cloud-sync of settings stops. The syncer is a **pure bridge** (doesn't own local persistence) → **deletes wholesale** in the surface phase; its ~18 `mock_initial_load` tests die with it (NOT rewritten). Net: reinforces "keep sqlite / amputate server" — settings stay local.

**Phase 1 scope (the surface deletion — LARGER than "delete 6 dead fns", entangled, defer to fresh context):**
- *Decision-free, deletable now:* `poll_for_updated_objects` + `handle_fetch_changed_objects_with_request_state` (dead in prod AND test); `OUT_OF_BAND_REQUEST_RETRY_STRATEGY` import; `MCPGalleryUpdated` variant; `user_profiles::clear_profiles`. (NOTE: deleting only these won't clear the grouped dead-code warning — the other 4 cluster methods are test-live.)
- *Test-live, need handling:* `on_changed_objects_fetched` + `handle_object_updates`/`handle_object_deletions` + `fetch_and_merge_environment_timestamps` are the seeding path for ~20 tests via `mock_initial_load` (18 in `cloud_preferences_syncer_tests.rs` — die with the syncer; notebook_tests:198 + update_manager_tests:158 — rewrite to seed locally via ONE helper that builds CloudModel like `lib.rs:1571` or via the kept `create`/`save_to_db` path).
- *Test discriminator:* asserts server behavior → delete; asserts a keep-feature using `mock_initial_load` as a seeding shortcut → rewrite to local seed.
- *Surface to delete together (where the field-level dead_code cascade finally resolves + binary shrinks):* `SyncQueue` drain machinery + `object_client` field; `Listener` (`get_warp_drive_updates` + helpers); `UpdateManager.object_client` + poll/abort fields; `ObjectClient` trait + `impl ObjectClient for ServerApi` + `server_api/object.rs` CRUD; `cloud_preferences_syncer` wholesale; the 3 `get_cloud_objects_client()` mount sites.
- `#[cfg(test)]`-gating the 4 test-live methods is an HONEST stopgap (NOT the banned `#[allow(dead_code)]`) that reaches 0 warnings + green-merges the behavioral change, but preserves the whole server type surface (no binary shrink, no cloud-surface removal) — only worth it to land a green milestone; the real win is the wholesale surface deletion above.

**⚠️ RESUME NOTE (verified 2026-05-29, end of session) — why Phase 1 is fresh-context work, and the exact entry point:**
- **KEEP the pub no-op methods** `start_polling_for_updated_objects` / `refresh_updated_objects` / `stop_polling_for_updated_objects` — they have **5 LIVE external callers** (`settings_view/teams_page.rs:964`, `settings_view/environments_page.rs:2021`, `workspaces/update_manager.rs:374`, `auth/mod.rs:222`, `listener.rs:330`). Deleting them cascades into those files. Same collapse-not-rip logic as the rest of the strip.
- **The private dead cluster** (`poll_for_updated_objects`@666, `handle_fetch_changed_objects_with_request_state`@747, `on_changed_objects_fetched`@770, `handle_object_updates`, `handle_object_deletions`, `fetch_and_merge_environment_timestamps`) interlocks with KEPT code: `stop_polling` (kept) → `abort_existing_poll`@656 → abort fields; and writes `should_poll_for_updated_objects`@212 which is READ only by the to-be-removed `poll` → after removal the field is write-only → fresh dead_code warning. So the cluster + fields + `abort_existing_poll` + `handle_network_status_changed`@729 (Online arm calls poll; Offline arm stop_dequeueing) must be reworked TOGETHER, not piecemeal.
- **Cleanest full Phase 1 (one connected pass, fresh context):** delete the private cluster; simplify `handle_network_status_changed` (drop poll arms); delete `abort_existing_poll` + abort fields + `should_poll` field; make the 3 kept pub methods truly empty `{}` no-ops (no field refs); delete `SyncQueue` drain + `object_client` + `Listener` body + `ObjectClient` trait + `impl for ServerApi` + `server_api/object.rs` CRUD; delete `cloud_preferences_syncer` wholesale (pure bridge) + its 18 tests; rewrite notebook_tests:198 + update_manager_tests:158 to seed locally (one helper). Then 3-gate green + binary shrink. THIS is where the field-cascade resolves cleanly — doing it as a single pass avoids the write-only-field churn that piecemeal hits.
- **`90890ad1` — SURFACE DELETION PASS DONE (2026-05-30):** Listener deleted; `UpdateManager.object_client` field gone + `new()` simplified; all mutation methods local-only (success emitted immediately, no server round-trip); `handle_model_event` + RTC handlers deleted; `resync_object`/`fetch_single_cloud_object` → no-ops; dead private functions removed (`revert_workflow_on_failed_move`, `store_metadata_update`, `remove_pending_object_action`, `remove_pending_action`). 28 files, −2,228 net LoC. Binary 875.7 → 873.9 MB (−1.87 MB). **3-gate 0/0/10-warn.** Deferred warnings: 4 AI territory (pre-existing) + 6 cloud-strip residue (see AGENTS.md).
- **`e44424f1` — AI territory dead-code sweep (2026-05-30):** Cleared 20 of 30 deferred warnings. 8 files, −345 net LoC, binary unchanged. Remaining 10 warnings: 4 test-only + 6 pre-existing cascade (see AGENTS.md).
- **`670f718c` — SyncQueue + ObjectClient + server_api/object.rs deleted (2026-05-30):** Deleted `server/sync_queue.rs` (1871 lines) + `server_api/object.rs` (1354 lines). Cascade: removed trait methods create/update_object_queue_item + send_create/update_request from CloudModelType/CloudObject/StringModel + all impls; removed serialized() from CloudModelType (kept in CloudStringObject for sqlite + AI context); update_manager RTC handlers + 3 enqueue blocks gone; lib.rs SyncQueue bootstrap gone; auth SyncQueue::clear gone; drive is_dequeueing → false; gql_convert ObjectUpdateMessage TryFrom removed; graphql schema dead fns removed; 21 test setups cleaned (SyncQueue::mock removed); server-sync-asserting tests deleted. 54 files, −5,624 net LoC. Binary 872.0 MB (unchanged — linker had already dead-stripped vtable since dequeue=false + UpdateManager.object_client removed in 90890ad1). 3-gate 0/0/0. Warnings: 8 default / 12 GUI (all pre-existing).

### shared_session strip — ✅ COMPLETE (`bfcb3ffe`, 94 files, −27,714 LoC, binary −11.9 MB, 3-gate green/0-warn)

Done via the batched-`python3`/`recast` method (NOT the Edit tool — per-call diagnostic injection blows the budget over hundreds of edits). Collapsed `SharedSessionStatus`/`IsSharedSessionCreator` to vestigial stubs in a ~95-line `terminal/shared_session.rs`; deleted both machinery dirs + the sharer-network hub in terminal_manager + all share-UI (view/pane_group/pane_impl/workspace/drive/tab/alt_screen/block_list) + AI auto-share + `warp_cli --share` + the session-sharing test suites. Cloud-mode ambient panes → local MockTerminalManager. `session_sharing_protocol` crate + `ai/blocklist/controller/shared_session.rs` kept (deferred AI-panel pass). Original IN-PROGRESS plan below for reference.

**Vestige cleanup — ✅ COMPLETE.** Slice 1 (`ab60b7d8`); UI-flag slices (all 5 ✅ — see below); `SharedSessionStatus` type removed (`feeb35ce`): collapsed ~90 `shared_session_status().is_*()` always-false call chains across 21 files, removed `SharedSessionStatus` enum + impl from `shared_session.rs`, removed field + getter + setter from `TerminalModel` (stubbed `is_shared_session_viewer()` → `false` directly), removed field + getter from `SessionNavigationData`, removed `available_to_session_viewer()` from `AgentToolbarItemKind` (always-true), removed `SharedSessionStatus` param from `render_cli_toolbar_item`/`render_toolbar_item`, deleted dead guards (sharer PTY-forward, obfuscation guard, viewer debug_asserts, `SharerSizeChanged` match arm). 21 files, −774 LoC. Binary 890.7 → 890.5 MB. 3-gate 0/0/0. `is_shared_session_viewer` cascade (`0ae8ad45`): collapsed all downstream reads of always-`false` stub — `TerminalModel::is_shared_session_viewer` + `TerminalView::is_shared_session_viewer` stubs; field/param from `DisplayChip`/`DisplayChipConfig`/`PromptDisplay`; field + setter from `AIContextMenuState` + `CommandPalette::View`; viewer-exclusion branches in `get_categories_for_mode`; param from `DataSourceStore::reset_search_mixer`; `Workspace::is_shared_session_viewer_focused` stub + all callers; `NotExecutedReason::WaitingOnSharer` + 3 executor guard blocks. 11 files, −165 LoC. Binary unchanged. 3-gate 0/0/0. Final vestiges (`e5658b03` + fix `e013a645`): `NewWorkspaceSource::SharedSessionAsViewer`; `ordered_terminal_events_for_shared_session_tx` field + set/clear methods + send blocks; full `response_initiator`/`shared_session_response_initiator` chain (`Exchange::response_initiator`, `RequestInput::shared_session_response_initiator`, `BlocklistAIController::shared_session_state`, `controller/shared_session.rs` module deleted, `BlockModel::response_initiator` trait method + impl); `participant_id: Option<ParticipantId>` param removed from all 7 `send_query`/`send_user_query_*` APIs (callers always passed `None`). Fix commit patched 6 missed call sites (stale incremental cache). 26 files total, −198 LoC. Binary unchanged. 3-gate 0/0/0.

**Slice 1 DONE (`ab60b7d8`, 3-gate 0/0/0):** deleted `SharedSessionActionSource` (0 consumers) + the always-empty `Tab::session_sharing_menu_items` + all 5 `WorkspaceAction` share variants (`OpenShareSessionModal`/`StopSharingSessionFromTabMenu`/`StopSharingAllSessionsInTab`/`CopySharedSessionLinkFromTab`/`OpenSharedSessionQrCode`) + Display/match-arm/handler sites + dangling imports. −122 LoC.

**⚠️ SCOPE FINDING (verified 2026-05-30): `session_sharing_protocol` is LOAD-BEARING for KEPT features — do NOT remove it.** `SessionId`/`SessionSourceType`/`SharedSessionSource`/`join_link` are consumed by ambient agents (`ambient_agents/spawn.rs`/`task.rs`), agent api, agent conversations, cloud objects (~30 files). `SharedSessionSource::ambient_agent(task_id)` tags live agent-task sources — not sharing. KEEP the crate + those types + the `terminal/shared_session.rs` stub's `SharedSessionSource`/`join_link`/`SessionId` exports.

**UI-FLAG SLICE — ALL 5 flags DONE.**
- ✅ **`SessionSharingAcls`** (`e5be4d3c`): 0 real readers — def + registration + Cargo feature gone.
- ✅ **`SharedSessionWriteToLongRunningCommands`** (`e5be4d3c`): sole reader `send_write_to_pty_events_for_shared_session` (always returned) + setter + `clear_*` + `write_to_pty_events_for_shared_session_tx` field + init + view.rs caller removed, then flag def.
- ✅ **`ViewingSharedSessions`** (`dc84212c`): `UriHost::SharedSession` variant + `"shared_session"` route + handler + arbitrary-path/window-behavior list entries; root_view join action registration + `open_shared_session_as_viewer` + `RootView::join_shared_session_in_existing_window`; the 2 horizontal-scrollbar flag terms (kept `is_active_viewer()` read-sites); deleted `test_view_only_session` + `mock_workspace_viewing_shared_session`. `NewWorkspaceSource::SharedSessionAsViewer` variant kept (still matched in workspace/view.rs; now constructed nowhere — folds into the read-site sweep).
- ✅ **`CreatingSharedSessions`** (`a3f63791`, 24 readers, −882 LoC): collapse all readers + remove def/DOGFOOD/registration/Cargo. Removed two self-contained shared-session-only UI subsystems — (1) close-session-confirmation dialog (`workspace/close_session_confirmation_dialog.rs` + `close_tabs` `dialog_source` param + confirm branch + `should_confirm_close_session` method/setting + `ToggleConfirmCloseSession` action + `ConfirmCloseSharedSessionWidget` + pane_group `CloseSharedSessionPaneRequested` event/emitter/handler + `Workspace::close_pane` + `is_close_session_confirmation_dialog_open` state) and (2) inline share banners (`terminal/view/inline_banner/shared_sessions.rs` + `SharedSessionBanners` enum + `shared_session_banner_state` field in TerminalView/block_list_element + render fns; `draw_border_above_block` → always-true). Plus app_menus Share items + CustomActions, `Indicator::Shared`/`sharing_color`, `/remote-control` slash cmd, experiment arms → no-op, `update_session_sharing_enablement` + its ServerExperiments subscription. **Agent-footer remote-control chips re-gated on `HOARemoteControl` alone (off by default → behavior preserved), deferring the agent-panel remote-control surface to the AgentSharedSessions unit.**
- ✅ **`AgentSharedSessions` + `HOARemoteControl` (`ba69a2ed`, −1386 LoC).** Both flags gone. The AI-panel shared-session/remote-control surface removed: the 3 controller flag-gated blocks (forward-to-viewers / cancel-to-viewers / status-bar cancel — all doubly-dead behind the `is_sharer()` stub); the **871-LoC `controller/shared_session.rs` collapsed to ~30 lines** (every handler had 0 live callers — kept only `get/set_current_response_initiator` + `get_sharer_participant_id`, called ~13× in the live request path, now always None); `input.rs` command-on-behalf + cancel-for-shared-session fns + `InputEvent::CancelSharedSessionConversation` + `CommandExecutionSource::SharedSession`; `Event::CancelSharedSessionConversation`; pty_controller arm; terminal_model `send_agent_response_for_shared_session`/`start_command_execution_for_shared_session`/replay methods/`ai_metadata_to_protocol`; the **entire agent-footer remote-control chip** (`AgentToolbarItemKind::ShareSession` + buttons + actions/events + `RemoteControlButtonTheme` + sync fn + `use_agent_footer` event chain); 4 now-orphaned `attachment_utils` fns. **Method: collapse the doubly-dead flag+`is_sharer()` blocks, then `cargo check` the dead module fns (0 callers) — most of the 871-LoC module was already dead, gated behind a runtime flag.**
- **KEPT as parked carriers (deep agent data-model — behavior-preserving, always None):** the `response_initiator` field on the agent Exchange/conversation model (`ai/agent/mod.rs`, conversation.rs, task.rs) + the `shared_session_response_initiator` `RequestInput` plumbing through controller. Removing them = touching the core agent exchange model + ~10 test sites; deferred with the SharedSessionStatus read-site sweep.
- **~105 `SharedSessionStatus` read-sites** (`shared_session_status().is_*()`) across block rendering = always-false dead branches; collapse last. Includes the leftover `NewWorkspaceSource::SharedSessionAsViewer` arms + the `response_initiator`/`shared_session_response_initiator` carriers above + terminal_model's `ordered_terminal_events_for_shared_session_tx` + `OrderedTerminalEventType` (pub, never set). KEEP the `SharedSessionStatus` type only if the field is still set somewhere; if all read-sites collapse, remove the type + `shared_session_status()` + field.

**Method that worked:** collapse dead `if flag.is_enabled() {…}` branches first (compiles fine with flag still defined — it's just fewer readers), bank green; remove flag def LAST once its readers hit zero. Cascades (banner module, dialog subsystem, URI route, REMOTE_CONTROL, close-session widget) surface as dead-code/unused-import warnings after the readers drop — use `cargo check --features gui` as the worklist oracle. For a subsystem that's reachable only through a now-permanently-false gate (close-confirm dialog, share banners), trace it to its event emitter — if the emitter is itself gated on `shared_session_status().is_sharer()` (always false), the whole chain is dead and removes cleanly. When a UI chip lives in kept agent code, re-gate it on the OTHER (off-by-default) flag rather than ripping the agent subsystem — defers cleanly, preserves default behavior.

### AI strip — current state (2026-06-07 session 61)

**Binary**: **659.2 MB** (`ce828549`). 3-gate **0/0/0 errors** throughout. Per-session detail lives in the `ai-strip-session-*` memory files (this plan stays high-level for 57+).

**S61**: coding_entrypoints orphan confirmed already-gone (stale row fixed). **Warpify tail Cluster A done** (`ce828549`, −0.49 MB): WarpifySettings singleton deleted, SSH wrapper collapsed to the generic `enable_legacy_ssh_wrapper` path (behavior-identical on defaults), ShowWarpifySettings action + tmux context flag removed. **Cloud-handoff increments 2+3 DONE** (`5721b391` C1 app-side + `f7d8d393` C2 daemon-RPC, −1104 LoC): the manual local-to-cloud handoff was already runtime-dead (empty `start_local_to_cloud_handoff` + `is_cloud_handoff_enabled*`→false). C1 removed the whole app surface (OpenLocalToCloudHandoffPane action, `/handoff` slash cmd, HandoffToCloud toolbar chip, the `&`-ampersand `HandoffComposeState`/`InputPrefixMode`/prefix_mode cluster in input.rs, the 4 is_*_handoff_enabled predicates + 2 should_force_disable settings, HandoffEntryPoint enum, CancellationReason::AutomaticCloudHandoff, CloudHandoff{Enter,Exit} input-source variants, did_add_handoff_chip_to_toolbar). C2 severed the `UploadHandoffSnapshot` daemon egress (deleted ai/blocklist/handoff/ + remote_server/handoff_snapshot.rs, neutered the daemon arm to an error response, removed the dead client wrapper + manager variant + InitialSnapshotToken; **KEPT the proto messages — schema-bound wire**). 3-gate 0/0/0 + remote_server crate/tests clean.

**DONE since session 56** (each its own `ai-strip-session-*` memory file + build-size rows):
- **S57** codebase-indexing/embeddings strip (Frontier A) — `944b0a70`+`f4082d6f`, −8.15 MB → 710.6 MB. Also: AIConversation keystone groundwork (light-type extraction + consumer neutering, see Frontier B below).
- **S58** VOICE feature (cpal/`voice_input` cargo feature) −2.05 MB; Teams backend (TeamClient CRUD) −3.05 MB; Referrals server fetch −0.59 MB.
- **S59** Warp-Drive sidebar panel −2.67 MB; block-sharing −2.20 MB; residual dead AI handlers; **Bedrock app-side + aws-sdk dep-drop −25.46 MB** (the big one) → 664.0 MB.
- **S60** top-bar/menu de-Warp + Account settings section removed; **SSH-extension phone-home** fully stripped (egress to app.warp.dev/download/cli + install machinery + RemoteTransport::install_binary); **cloud-handoff increment 1** (dead auto-cloud-handoff subsystem + orphaned SystemStats sleep/wake cascade); **WARPIFY feature fully removed** (10 commits `15a0f0ed`→`e6ef17e8`, −2.0 MB, GUI-verified blocks render).

**REMAINING (mapped in memory files):**
- **warpify tail Cluster B** (no warnings — inert, NOT dead-code-flagged) — the ANSI OSC/DCS subshell/ssh parser (`model/ansi/dcs_hooks.rs` `DProtoHook::{InitSubshell,InitSsh,SshTmuxInstaller,TmuxInstallFailed,RemoteWarpificationIsUnavailable}`) + the model-events (`InitSubshell`/`InitSsh`/`SourcedRcFileInSubshell`/`SshTmuxInstaller`/`TmuxInstallFailed`/`RemoteWarpificationIsUnavailable`/`TmuxControlModeReady`/`DetectedEndOfSshLogin`) + `TmuxInstallationState`. These are fully plumbed (parser→event→ModelEvent→**no-op** view handler) so they produce no warnings; removing them is pure SSH-escape-sequence-parser surgery with **zero warning payoff** and **live edges** (`TmuxControlModeReady`→`pty_controller.rs:159`, `DetectedEndOfSshLogin` drives the SSH-login-detection state machine in terminal_model.rs). **KEEP `SubshellSource`** (warpify/mod.rs — drives the generic subshell FLAG, NOT dead). Defer Cluster B to an ssh-testable session, or treat as an optional generic "SSH DCS parser slim-down" rather than warpify cleanup → `ai-strip-session-60-warpify.md` + `ai-strip-session-61.md`.
- ~~cloud-handoff increment 2/3~~ ✅ DONE S61 (`5721b391`+`f7d8d393`). Cloud-handoff feature fully stripped (auto inc1 S60 + manual/RPC inc2+3 S61). `KEEP handoff_compose`-era note in `ai-strip-session-60-cloud-handoff.md` was wrong: `HandoffComposeState` was the `&`-ampersand cluster, all dead → removed.
- ~~Frontier B — `ai/agent/` AIConversation keystone~~ ✅ **DONE in session 58** (`ed89397e` mass cut + `302a2ba1` persistence cluster, 704.3→697.7 MB). `app/src/ai/agent/` no longer exists; zero `AIConversation` refs. See KEYSTONE §"Step 4 increment 5/5b" below. (The "NEXT" tag here was stale through session 61 — verified gone + corrected.)
- ~~cloud_preferences Increment B~~ ✅ effectively DONE — `settings/cloud_preferences.rs` + `CloudPreference`/`CloudPreferenceModel` + `ServerCloudObject::Preference` are already gone (removed across the s56–59 cloud-object/Warp-Drive strips). Remaining `JsonObjectType::Preference` (warp_server_client + sqlite deserialize→None) + `JsonPreference` (cynic graphql) are **schema-bound/DB-compat → KEEP** (same precedent as Bedrock ManagedSecretValue + LLMModelHost wire variants). Stale `CloudPreferencesSyncer` doc-comment in initializer.rs removed s61.
- De-brand rename (`warp*` → brand) LAST, after cloud surface gone. (`coding_entrypoints/` orphan — already gone at `3e87396f`; row above corrected session 61.)

---

## NEXT TARGETS — mapped 2026-06-06 (two frontiers, full investigation)

### Frontier A — Codebase-indexing / embeddings ✅ DONE (session 57, `944b0a70` + `f4082d6f`, −8.15 MB / −16,501 LoC)
**Done in 2 phases** (sever consumers → delete dead engine + graphql ops + flags). Was bigger than mapped: woven into code_page.rs (LSP UI shared the same render path — kept LSP/rules/code-review, renamed category "Language Servers"), init_project (removed the CodebaseContext step, kept the rest), persisted_workspace (kept LSP + workspace-metadata persistence), the daemon (server_model.rs neutered to "not enabled" stubs, generic remote_server proto crate kept). **KEY trap caught:** the `codebase_indices` sqlite table + `Upsert/DeleteCodebaseIndexMetadata` ModelEvents are misleadingly named — they persist generic LSP WorkspaceMetadata (KEEP). `codebase_context_enabled`/`is_codebase_context_enabled` kept (used by surviving file outline + slash-command data source). Original verdict (for reference): cloud-coupled, sends code fragments + merkle hashes to Warp GraphQL servers; isolated `StoreClient` seam.
- **DELETE outright:** `crates/ai/src/index/full_source_code_embedding/` (~11.8K LoC: merkle_tree/, chunker/, codebase_index.rs 2382, manager.rs 1332, sync_client, snapshot, store_client, manager + tests) · `app/src/ai/codebase_auto_indexing.rs` (112) · `app/src/remote_server/codebase_index_model.rs` (832) + `codebase_index_status.rs` (211) · 7 GraphQL op files in `crates/graphql/src/api/full_source_code_embedding/` (queries codebase_context_config/get_relevant_fragments/rerank_fragments/sync_merkle_tree; mutations generate_code_embeddings/populate_merkle_tree_cache/update_merkle_tree) · AIClient methods `update_merkle_tree`+`generate_code_embeddings` (decls ai.rs:124-140, impls 263-337) + `StoreClient for ServerApi` impl (ai.rs:537-706) + structs EmbeddingConfig/IntermediateNode/NodeHash/ContentHash/RepoMetadata/Fragment/CodebaseContextConfig (in store_client.rs).
- **TRIM (remove index hooks, keep host):** lib.rs:1582-1600 (singleton reg) · persisted_workspace.rs:236-696 (index_repo + index calls; keep project-rules) · auth/mod.rs:166 (reset_codebase_indexing) · remote_server/server_model.rs:795-998 (mostly already stubbed not_enabled) · settings_view/code_page.rs (232,582-716,…) · directory_color_add_picker.rs (index-event subscribe) · terminal/view.rs:16812 (IndexProjectSpeedbump action) · terminal/view/init_project/ (only the CodebaseIndexManager hooks — keep InitProject feature) · settings/code.rs:14-32 (codebase_context_enabled/auto_indexing_enabled) · features.rs:88-89,107-108 · crates/ai/src/index/mod.rs (drop `pub mod full_source_code_embedding`) · 8 test files (pane_group/mod_tests, terminal/input_tests, code/file_tree/view/view_tests, workspace/view_tests, test_util/terminal, integration_testing/codebase_context/step).
- **KEEP (do NOT touch):** `crates/ai/src/index/file_outline/` + `app/src/ai/outline/` (LSP symbol outline, zero embedding deps) · `crates/repo_metadata/` + the `index/mod.rs` re-exports of it (file-tree/gitignore infra — "repo_metadata" the CRATE, not the embedding `RepoMetadata` struct) · `index/locations.rs` + THREADPOOL/build_outline rayon infra.
- **Order:** drop index hooks in app/src callers → delete StoreClient impl + AIClient methods in ai.rs + the 7 graphql ops → delete the engine subtree + drop the `pub mod` line → fix the 8 test files. 3-gate + watch cross-crate (ai/graphql) compile via `-p warp`.

### Frontier B — Core AI agent `ai/agent/` (bigger, tangled; do AFTER A)
17,333 LoC tangled core (`conversation.rs` 3896, `mod.rs` 2701, `api/convert_conversation.rs` 2030 + tests, `convert_from.rs` 1024, `task.rs` 1063, `task_store.rs` 351). `api/impl.rs`+tests already empty stubs.
- **3 light types MUST be extracted first (load-bearing for SURVIVING features):** `ConversationStatus` (conversation.rs:3792 — CLI-agent status display: `terminal/cli_agent_sessions/mod.rs:24` maps CLIAgentSessionStatus→ConversationStatus; rendered by conversation_status_ui/agent_icon/vertical_tabs) · `AIConversationId` (conversation.rs:3610 — persistence/session-restore serialization, pane_group, terminal/input) · `ServerConversationToken` (api.rs:16 — root_view/workspace/uri/terminal, 6+ files).
- **The `AIConversation` aggregate (conversation.rs:123) is the keystone** — transitively owns TaskStore, task.rs, todos, comment, linearization, conversation_usage_metadata, and the entire `api/convert_*` MAA-proto machinery (~5.7K LoC, zero surviving external callers but internally entangled). `AIAgentHarness`/`ServerAIConversationMetadata` are near-dead (only server_api/ai.rs GraphQL convert + pane_impl.rs). `AIAgentTodo`/`AIAgentTodoList` zero external consumers.
- **Removal order:** (1) neuter dead AIConversation construction paths (server_api GraphQL TryFrom, pane_group child-agent restore already runtime-dead at mod.rs:2146/2109), (2) extract the 3 light types into a small standalone module, (3) the whole conversation/mod/task*/api cluster then collapses as one unit. **CLI-agent-sessions (surviving) need ONLY `ConversationStatus`.**

**STEP 1 DONE (session 57, `597d9452`):** extracted the 4 light types (AIConversationId, ConversationStatus, StatusColorStyle, ServerConversationToken) into `ai/agent/conversation_types.rs`; conversation.rs + api.rs `pub use` re-export them so all ~25 external import paths resolve unchanged. Pure relocation, 3-gate 0/0/0, binary flat.

**STEP 2 PARTLY DONE (session 57, batches 6–8 `8f855193`/`094f804f`/`703d4003`):** all peripheral AIConversation consumers NEUTERED — server_api/ai.rs (both ServerAIConversationMetadata TryFroms + convert_harness/convert_usage_metadata), pane_impl.rs (chrome methods now return None/default sans AIConversation; kept ConversationStatus), agent_icon.rs (dropped server-metadata fallback), settings/ai.rs (handoff `_for_conversation` + is_orchestration_conversation), conversation_navigation (from_ai_conversation), command_palette data_source (fork stub). **REMAINING BLOCKER before keystone delete = the `ConversationRestoration` dataflow** (82 refs / 10 files: pane_group/mod.rs restore + create_session, terminal mock/local/remote managers, terminal/model/blocks.rs, block_list_element.rs, terminal/view.rs, workspace/view.rs) + `RestoredAgentConversations` model (lib.rs singleton + restored_conversations.rs). It's threaded through terminal/pane creation signatures, so it must be removed as ONE atomic green commit (can't partially remove). RestoredAgentConversations::new() already discards input → the whole restore path is runtime-dead; removing it = dropping ConversationRestorationInNewPaneType + the restoration params from create_session/create_pane + the take/get_conversation calls (pane_group ~1383/1401/2125/2179) + create_hidden_child_agent_pane + new_for_conversation_transcript_viewer + create_conversation_viewer. After that, AIConversation has zero external refs → delete the keystone (conversation.rs AIConversation + task*/api/convert*/todos/comment/linearization + ServerAIConversationMetadata + AIAgentHarness). **Leftover dead variants to sweep with the keystone:** ConversationAction::Fork (matched in handler, never constructed now), WorkspaceAction::ForkAIConversation + fork_ai_conversation stub.

**STEP 2/3 — original verified classification (session 57 investigator, AIConversation has ZERO production constructors; all `AIConversation::new*` are test-only).** External `AIConversation`-type refs to neuter (then delete cluster):
- (DEAD restore/viewer — remove fn + dead caller) `restored_conversations.rs` get/take_conversation* (return from always-empty map; `new()` discards input) · `pane_group/mod.rs` create_hidden_child_agent_pane (~2216), new_for_conversation_transcript_viewer (~2550), create_conversation_viewer (~4439) — all reached only from runtime-dead restore/loading paths.
- (returns None/false — remove fn + callers) `command_palette/conversations/data_source.rs` selected_conversation_in_focused_pane (always None) · `pane_impl.rs` selected_conversation_for_user_facing_chrome (always None) + display_title_for_chrome + `.and_then(AIConversation::server_metadata)` (~564/570/652) · `settings/ai.rs` is_cloud_handoff_enabled_for_conversation/is_ampersand_handoff_enabled_for_conversation/is_orchestration_conversation (~1592/1614/1924, all return false).
- (dead GraphQL convert — remove) `server_api/ai.rs` `impl TryFrom<warp_graphql::ai::AIConversation> for ServerAIConversationMetadata` (~332, never called) — also frees `AIAgentHarness`/`ServerAIConversationMetadata` (no other external users; `pane_impl.rs` server_metadata chain is the dead-None one above).
- (metadata extractor — repoint or delete) `conversation_navigation/mod.rs` from_ai_conversation (~60, only reads nav metadata of historical/dead conversations).
- (LIVE only under `agent_mode_evals` feature) `integration_testing/agent_mode/llm_judge/mod.rs` format_conversation_history_for_llm_judge + judge — gated; either drop with the cluster (feature is off in normal builds) or keep behind cfg if it must compile under that feature.
- `command_palette/view.rs` ForkAIConversation dispatch + `workspace/view.rs:8618` fork_ai_conversation empty stub — remove the action+stub. `persistence` DeleteAIConversation/`delete_ai_conversation` take only a String id — KEEP (generic persistence).
- `api/impl.rs` is a 0-byte stub. No external `convert_conversation`/`convert_from` callers. **After step 2, delete as ONE unit:** conversation.rs (AIConversation + impls, keeps only the re-export line which moves to mod.rs) + mod.rs orchestration + task.rs/task_store.rs/todos/comment/linearization + api/convert_conversation/convert_from/impl + their tests (~14K LoC). Expect a real binary shrink only at the final deletion (AIConversation is currently dead-stripped-ish but the convert machinery + task store may be partially live-linked via the agent module surface).

**Warp Drive sidebar PANEL stripped (2 phases, −2.67 MB, ~9000 LoC):** P1 `644c340b` severed the entry point (DrivePanel registration + ToolPanelView::WarpDrive + ~60 sites in left_panel/view.rs + lib.rs init). P2 `d3e93af3` deleted the dead cluster (index.rs 5332 + panel.rs + items/ + 2 dialogs + tests), relocated the 2 generic types it defined (DriveIndexVariant, WarpDriveItemId → drive/mod.rs, ~200 refs), removed dead to_warp_drive_item from the generic CloudObject trait + 8 impls. **KEPT generic:** cloud_object/ (CloudObjectTypeAndId/CloudModel/GenericCloudObject/JsonObjectType), drive/folders, drive/workflows (modal editors), export, settings, cloud_object_styling, drive_helpers, OpenWarpDriveObject*, DriveObjectType. **LESSON:** the investigator initially mis-classified index.rs/items/ as "drive-internal-only" — but DriveIndexVariant/WarpDriveItemId/DriveIndexAction were entangled with generic cloud_object persistence + env_vars/mcp views (~200 refs). Verify trait-impl usage (grep `.method(` call sites, not just `impl`) before deleting a file with trait impls. OpenWarpDrive/ToggleWarpDrive workspace actions left as no-op seams (still dispatchable; could remove the action variants later).

**Dead TypedActionViews DONE (`118b87bb`, −0.09 MB):** AgentTodosPopupView (empty stub, init-only keybinding, never instantiated) + DeleteConversationConfirmationDialog (zero-caller show_*(), dead UI — trigger removed in prior strips). Removed only the dialog-specific flag from the generic current_workspace_state struct.

**harness-availability + auth-secret FTUX subsystem DONE (`2f380d2a`, −2.89 MB, −2428 LoC):** deleted ai/harness_availability.rs (HarnessAvailabilityModel) + ambient_agent auth_secret_ftux_view/dropdown (TypedActionView regs → real shrink) + ai/auth_secret_types.rs + ai/harness_display.rs + entire ai/cloud_agent_settings.rs (all 7 fields zero-caller after model-picker/orchestration strips) + workspace create_auth_secret_modal (already-unreachable dead UI — its OpenCreateAuthSecretModal dispatcher was gone) + server get_available_harnesses. Kept ActionPermission/WriteToPtyPermission/ComputerUsePermission + convert_harness. **LESSON:** harness_display.rs ALSO held `From<AIAgentHarness> for Harness` + `PartialEq<Harness>` used by surviving conversation.rs (trait impls — invisible to symbol greps); relocated them to conversation.rs (caught via E0308). Always build after deleting a file with trait impls.

**execution_profiles cleanup DONE (`e5c1bd54`):** deleted the dead AIExecutionProfile data cluster (struct + cloud-object StringModel/JsonModel impls + CloudAIExecutionProfile aliases + RunAgentsPermission + AskUserQuestionPermission + AIExecutionProfileInfo + zero-caller model methods). Kept ActionPermission/WriteToPtyPermission/ComputerUsePermission, ClientProfileId + AIExecutionProfilesModel singleton.

**`ai/llms.rs` subsystem strip — DONE (4 phases, `a446aa0d`→`0e2fe341`, −3.0 MB, ~3500 LoC):**
- P1 `a446aa0d`: repoint 9 `LLMId` imports to the `ai` crate (LLMId is re-exported, not owned by llms.rs).
- P2 `91aa75d3`: relocate `LLMModelHost` enum → `workspaces/workspace.rs` (baked into LlmSettings serialization; only cross-cutting type). llms.rs re-exported it until P4.
- P3 `d53b3872`: delete inline model-picker UI `terminal/input/models/` (1493 LoC) + `OpenModelSelector` action + `InputSuggestionsMode::ModelSelector` + `/model` command + `InlineMenuType::ModelSelector`. Reached only via /model slash command (no keybinding/toolbar). Two `add_typed_action_view` registrations freed → real shrink.
- P4 `0e2fe341`: delete `ai/llms.rs` + `llms_tests.rs` + `LLMPreferences` singleton (lib.rs + 4 test regs + input.rs sub + update_manager sync + pane_group restore + terminal_pane write→None) + `server_api/ai.rs` `get_feature_model_choices` + ~290 LoC GraphQL conversions.

**NEXT (session 58):** Frontier B — **Core AI agent `ai/agent/`** (17K LoC tangle; see "Frontier B" above). Extract the 3 light types (ConversationStatus/AIConversationId/ServerConversationToken) first, then the AIConversation keystone collapses. **LESSON:** `create_agent_task` (server_api/ai.rs) shows dead in the DEFAULT gate but is alive via `pane_group/pane/local_harness_launch.rs` (local_fs-gated) — do NOT delete (session-28 feature-gate trap). `JsonObjectType` is defined in the `warp_server_client` crate — variant removal there is outside the `-p warp` gate (run `cargo check -p warp_server_client` if touched). `managed_secrets` crate no longer exists (removed earlier); the post-FeatureFlag-sweep crate to also check is `warp_cli`.

**Session 56 earlier work:**
- Fixed 3 GB `git push` failure: committed Cargo `app/src/target/` build artifacts (450–534 MB blobs) had bloated history; `git filter-repo --path app/src/target --invert-paths` stripped them (3 GB → 15 MB pack), force-pushed clean. Added `target/gate-*` dirs to `.gitignore` (`027c34f5`).
- `bf386fa1` (−0.63 MB): `ai/agent_conversations_model.rs` (867) + `entry.rs` (521) + orphan tests (2240) + `conversation_utils.rs` deleted. AgentConversationsModel singleton removed from lib.rs + 4 test regs. Callers fixed: agent_icon (task-data lookup dropped), conversation_status_ui (AgentRunDisplayStatus impl dropped), pane_group (LeafContents::AmbientAgent restore collapsed to plain terminal + pending-restoration plumbing removed), slash_commands, auth log_out. NOTE: plan's "32.1K/78.4K lines" was wrong — actual 867+521+2240.
- `64e4e9f5` (−1.13 MB): exposed-dead ambient task-fetch chain deleted. task.rs: AmbientAgentTask, RunExecution, AmbientAgentLiveSessionState, AmbientAgentTaskState, TaskStatusMessage, TaskStatusErrorCode, RequestUsage, TaskPrincipalInfo + helpers (kept AgentConfigSnapshot, HarnessConfig, AgentSource, AmbientAgentTaskId, normalize_orchestrator_agent_name, cancel_task_*). ai.rs: list_ambient_agent_tasks + get_ambient_agent_task + ListRunsResponse + TaskListFilter + build_list_agent_runs_url. ai_tests.rs: 15 tests. artifacts/mod.rs: deserialize_artifacts. Warnings dropped to 90/61/91 — below session-start baseline 91/61/92.

**LESSON (session 56):** `normalize_orchestrator_agent_name` was flagged dead in the default gate but is used by `pane_group/pane/local_harness_launch.rs:66` (a `local_fs`-gated file). Caught it via the injected diagnostic stream when the import broke — restored it. Re-confirms session-28 rule: feature-gated callers hide real users from the default gate; verify all 3 gates.

**Next targets (in order of binary impact):**
1. `ai/llms.rs` (39.0K) — LLM listing/management
2. `ai/harness_availability.rs` (11.1K) — harness availability
3. Small Warp-AI TypedActionView views: `AgentTodosPopupView` (56 lines), `DeleteConversationConfirmationDialog` (175 lines)
4. `drive/` + `cloud_object/` — Warp Drive cluster (21K+ lines, 144 callers — very complex)

**Session 55 completed:**
- `f12b1a07`: AgentToastStack deleted (462 lines).
- `c4e3259e`: search/ai_context_menu/ deleted (7472 lines) + AIContextMenu excised from editor/terminal/input. −3.6 MB binary.
- `6783a8fa`: AIDocumentView cluster deleted — ai_document_view.rs + ai/document/ model + ai_document_pane.rs. −2.2 MB binary. AIDocumentModel stub kept (used by terminal/input/plans/data_source.rs).
- `8aeb0f57`: ai/facts/ cluster deleted — ai/facts/ (2097 lines) + drive/items/ai_fact.rs + ai_fact_collection.rs. −1.9 MB binary.
- Dead code cleanup: deferred_panes parameter chain removed from restore_pane_leaf/restore_pane_tree, process_deferred_panes deleted; tooltip_text + terminal_view_id removed from UDI.
- Warnings: 90/60/91 (6 above previous floor; pre-existing dead code in KEEP modules exposed by deletions).

**Session 53 state (for reference):**
- Binary: 736.7 MB. Warnings: 84/56/85. Latest commit `3e442c75`.
- Session 53: ambient_agent/model.rs + progress_ui_state.rs deleted (AmbientAgentViewModel); convert_to.rs deleted (Warp MAA serialization). −0.12 MB.
- Session 52: spawn infrastructure deleted. −0.26 MB.

**Previous state (`0d24336f`, 2026-06-01 session 4):**

Binary: 794.3 MB (unchanged — handler bodies were already dead-stripped). 3-gate **71/86/71**. Commits `0d24336f` + `7a710de6`.

**Phase G-preview DONE (`1f600e66`):**
- `ai_controller: ModelHandle<BlocklistAIController>` removed from TerminalView struct (Phase G-preview goal achieved).
- Also removed: `passive_suggestions_models`, `get_relevant_files_controller`, `cli_subagent_controller`, `agent_todos_popup`, `is_todo_popup_visible` (direct dependents of ai_controller).
- Kept as static-default fields (no longer updated by deleted handlers): `ai_render_context`, `cli_subagent_views`, `pending_user_query_view_id/kind`, `queued_prompt_callback`, `conversation_completed_callbacks`, `usage_footer_view_ids`.
- 35 AI-only methods deleted from terminal/view.rs (28 + 7 via recast_structural).
- ai_controller + cli_subagent_controller created as LOCAL-ONLY variables in new() and passed to Input::new() so Input layer (Phase F) works unchanged.
- Stub getters `ai_context_model()`, `ai_input_model()`, `active_conversation_id()`, `stop_local_agent_conversation()`, `remove_pending_user_query_block()` added for external callers.
- All `ai_controller.update(...)` call sites in workspace/view.rs, code_review, pane_group, ai_document_view, etc. removed or no-op'd.
- `controller_tests.rs` test disabled with `#[ignore]` (Phase F work).

**Phase G handler-body removal DONE (`0d24336f`, session 4):**
- Deleted 20+ handler method bodies in `terminal/view.rs` that called `.update()` on `ai_context_model`, `ai_input_model`.
- Removed `CLISubagentController` creation + `cli_subagent_views` HashMap from TerminalView struct.
- Replaced all `is_ai_input_enabled()` branches with unconditional shell-mode behavior.
- `ai/agent/api.rs`: removed `BlocklistAIPermissions::as_ref` checks → always true.
- `ai/agent/todos/popup.rs`: stripped AI context/history deps, renders `Empty` now.
- `lib.rs`: removed dead singleton registrations (`BlocklistAIHistoryModel`, `BlocklistAIPermissions`, `OrchestrationEventService/Streamer`, `TaskStatusSyncModel`, `LocalSharedSessionLinkModel`, `block::status_bar::init`).
- `ai/mod.rs`: fixed init() paths. `ai/agent/redaction.rs`: fixed `secret_redaction` import.
- `ai/blocklist/input_model_stubs.rs`: deleted; real types (`InputConfig`, `InputType`, `InputTypeAutoDetectionSource`) now live in new `input_config.rs`.

**Phase G struct-field removal BLOCKED — `ai_context_model`/`ai_input_model`/`ai_action_model` still in TerminalView + Input:**
- These fields CANNOT be removed yet: direct struct-field access occurs in submodules:
  - `terminal/view/pane_impl.rs:210,644` — direct `self.ai_context_model.as_ref(app)...` access
  - `terminal/view/context_menu.rs:158` — direct `self.ai_action_model` access
  - `workspace/view.rs`, `code_review_view.rs`, `ai_document_view.rs` — call `terminal_view.ai_context_model()` getter
  - `terminal/input/` data sources — call `Input.ai_context_model()`
- Fix required before field deletion:
  1. `pane_impl.rs:210,644` — switch from direct field access to singleton pattern on `BlocklistAIContextModel`
  2. `context_menu.rs:158` — replace `self.ai_action_model` direct field with no-op / singleton call
  3. `workspace/view.rs`, `code_review_view.rs`, `ai_document_view.rs` callers of `terminal_view.ai_context_model()`
  4. `terminal/input/` data sources calling `Input.ai_context_model()`
  THEN delete fields + constructors from `terminal/view.rs` and `terminal/input.rs`.

**CRITICAL LESSON (session 4):** "Delete-and-fix all at once" fails for `inline_action/` + stubs cascade — 65+ files with errors, 228 errors total when attempted. Some callers use DIRECT FIELD ACCESS (not method calls, not getters) to `TerminalView.ai_context_model`. Those must be fixed to singleton-pattern calls BEFORE the field can be removed.

**CRITICAL PHASE G-PREVIEW LESSON (from this session):**
- `ai_render_context` and `cli_subagent_views` are deeply woven into rendering code (block_list_element.rs). Do NOT remove them from the struct — keep as static-default fields. Only the event handlers that UPDATED them need to be deleted.
- The "local-only variable" pattern: create ai_controller + cli_subagent_controller in new() as local vars, pass to Input::new() — keeps Input working without the fields being in the struct.
- When adding back recast-deleted getter methods as stubs, return `&ModelHandle` (by reference) not by clone — external code chains `.as_ref(ctx)` which works on both.
- `Input::new()` signature keeps all 18 params (including ai_controller + cli_subagent_controller) — removing them from Input is Phase F work.

**Done earlier this session:**
- `ai/blocklist/agent_view/` deleted (`94c6be3f`, −7.80 MB). All AI agent view UI gone.
- `terminal/view/agent_view.rs`, `load_ai_conversation.rs`, `use_agent_footer/`, `pending_user_query.rs` deleted.
- `terminal/input/agent.rs`, `terminal/input/conversations/` deleted.
- `ai/active_agent_views_model.rs` deleted.
- Relocated types: `AgentToolbarItemKind` → `context_chips/toolbar.rs`; `AgentViewState/EntryOrigin/render_block_container` → `terminal/view/agent_view_state.rs`; `AgentInputButtonTheme` → `terminal/view/ambient_agent/button_theme.rs`.
- `TextLocation` relocated to `util/text_location.rs`; `LinkActionConstructors` to `util/link_detection.rs` (`73dd201c`).

**Phase F steps 1–3 DONE:**
- Step 1 `202b79e0`: controller + passive_suggestions deleted. Relocated: SessionContext, RequestInput, ResponseStreamId, ClientIdentifiers.
- Step 2 `c42b65cb`: action_model deleted. Stub: `ai/blocklist/action_stubs.rs` (400+ lines).
- Step 3 `0576df88`: orchestration cluster deleted (orchestration_events/streamer/topology/links, task_status_sync, local_shared_session_link). Stub: `ai/blocklist/orchestration_stubs.rs`.

**What remains in `ai/blocklist/`:** `block.rs` (6481 lines) + `block/` dir + `inline_action/` dir + `permissions.rs` + `persistence.rs` + leaf files. All model layers stubbed: action_stubs.rs + orchestration_stubs.rs + context_model_stubs.rs + input_model_stubs.rs + history_model_stubs.rs. Binary flat until block/ callers deleted.

**Phase F steps 4–6 DONE** (2026-05-31 session):
- Step 4 (`b3ae3d79`): `context_model.rs` (924 LoC) + tests deleted → `context_model_stubs.rs`. block_context_from_terminal_model kept intact. 3-gate 0/0/0.
- Step 5 (`97a134ef`): `input_model.rs` (795 LoC) deleted → `input_model_stubs.rs`. detect_and_set_input_type no-op; InputConfig/InputType kept real. 3-gate 0/0/0.
- Step 6 (`125c0f72`): `history_model.rs` (2858 LoC) + `history_model_tests.rs` (2532 LoC) + `conversation_loader.rs` (663 LoC) deleted → `history_model_stubs.rs`. 81 methods no-op; `#[path]` redirect keeps 72 external `::history_model::` imports unchanged. 3-gate 0/0/0.

**CURRENT STATE (2026-06-02 session 18 COMPLETE):**
- 3-gate: **0/0/0**
- Binary: **758.3 MB** (758,324,264 B; −0.60 MB vs session 17 end 758.9 MB)
- Session 18 commits: `98278b0f` (bonus_grant_notification_model + 4 dead settings, −0.19 MB), `9d652a94`+`e6091b68` (anon sign-up banner + 3 dead InlineBannerType variants, −0.31 MB), `d6cadb90` (agent_mode_setup banner, −0.10 MB)
- Session 19 handoff at `/tmp/session19-handoff.md`

**DELETION CRITERIA (established session 18):**
Delete only if the code exclusively serves Warp's proprietary cloud AI (Warp agent, billing, credits, auth). KEEP anything that could serve vendor CLI agents (claude/codex/gemini) or generic LLM providers (AWS Bedrock) — even if originally added for Warp AI. Verified KEEP: aws_bedrock_login + aws_cli_not_installed banners (generic AWS LLM provider infra).

**DONE session 18:**
- `bonus_grant_notification_model.rs` (132 LoC) — Warp billing credits granted toast
- 4 dead GeneralSettings fields: agent_mode_onboarding_block_shown, free_tier_limit_hit_modal_dismissed, did_non_anonymous_user_log_in, bonus_grants_shown
- `inline_banner/anonymous_user_ai_sign_up.rs` (240 LoC) — prompts anonymous users to sign up for Warp cloud AI
- `anonymous_user_ai_sign_up_banner_shown` setting from GeneralSettings
- 3 dead InlineBannerType variants: PromptSuggestions, SharedSessionStart, SharedSessionEnd
- `inline_banner/agent_mode_setup.rs` (96 LoC) — prompts to set up Warp agent mode for a repo
- `agent_mode_setup_banner_shown_for_repo_paths` setting from AISettings

**DONE session 17: Delete cloud_agent_capacity_modal + free_tier_limit_hit_modal + codex_modal**
- `workspace/view/cloud_agent_capacity_modal/` (448 LoC) — AI cloud capacity/credits limit modal
- `workspace/view/free_tier_limit_hit_modal.rs` (464 LoC) — free-tier AI quota limit modal
- `workspace/view/codex_modal.rs` (291 LoC) — GitHub Copilot integration prompt modal
- All 3 were live-linked via `ctx.add_typed_action_view` in `Workspace::new()` → real binary shrink
- Full call chains excised: workspace mod/field/new/render, util WorkspaceState fields, pane_group Event variants, terminal_pane handlers, terminal::view Event variants, ambient_agent ShowCloudAgentCapacityModal emit chain, root_view codex registration + fns
- `show_out_of_credits_modal` simplified: drops `if is_on_paid_plan` branch (no modal to open), always refreshes usage
- `FreeTierLimitCheckTriggered` was never emitted (dead since addition) — whole chain removed
- `AgentViewState::CodexModal` dead variant removed from agent_view_state.rs

**CURRENT STATE (2026-06-02 session 16 complete):**
- 3-gate: **0/0/0**
- Binary: **759.6 MB** (−2.97 MB total this session: −1.17 MB conversation_list + −1.02 MB launch_modals + −0.81 MB build_plan_migration_modal; 759,553,848 B measured)
- Session 16 commits: `7d02ab62` (conversation_list), `15a601f6` (launch_modals), `b5d15bc3` (build_plan_migration_modal)
- Session 16 LoC removed: ~5,449 net (−1,992 + −2,333 + −1,124)

**DONE session 16: Delete conversation_list + launch_modals + build_plan_migration_modal:**
- `workspace/view/conversation_list/` (4 files −1,775 LoC + callers): ConversationListView AI inbox panel
- `launch_modal/` + `openwarp_launch_modal/` + `orchestration_launch_modal/` (3 dirs −2,042 LoC + callers): 3 one-time AI launch modals
- `build_plan_migration_modal.rs` (870 LoC + callers): cloud billing migration modal; OneTimeModalModel collapsed to 25 LoC no-op stub

**DONE session 15: Delete `ai/agent_management/` entire directory (~5900 LoC)**

Live-linked via `ctx.add_typed_action_view(|ctx| AgentManagementView::new(...))` at `workspace/view.rs:2264`.
Also: `AgentNotificationsModel` singleton registered at `lib.rs:1475`.

Files to delete: `ai/agent_management/mod.rs` + `view.rs` (2162) + `cloud_setup_guide_view.rs` (700) + `agent_type_selector.rs` (475) + `agent_management_model.rs` (357) + `details_action_buttons.rs` (285) + `notifications/` (1922 LoC).

External callers:
- `ai/mod.rs` — remove mod + init call
- `lib.rs:137,1475` — remove AgentNotificationsModel import + singleton registration
- `app_state.rs` — remove `PersistedAgentManagementFilters` struct + `WindowSnapshot.agent_management_filters` field
- `workspace/view.rs` (~82 refs) — imports, 3 struct fields, construction block, 2 handler methods, render paths
- `workspace/view/right_panel.rs` — `is_agent_management_view_open` field + `set_agent_management_view_open` method
- `workspace/header_toolbar_item.rs` — `AgentManagement` variant + 7 match arms
- `workspace/view/launch_modal/oz_launch.rs` — `OzLaunchSlide::AgentManagement` variant + ~15 match arms
- `persistence/sqlite.rs` — 2 `agent_management_filters` field entries in struct literals
- `persistence/sqlite_tests.rs` — 3 `agent_management_filters: None` entries

**KEEP `AgentManagementFilters` in `agent_conversations_model.rs`** — used by `entry.rs` which is live non-management code.

Handoff at `/tmp/session15-handoff.md`.

**Session 14 deletions done:**
- `ai/conversation_details_panel.rs` (2086 LoC) — Warp cloud AI conversation details side panel
- conversation_status_ui.rs + conversation_utils.rs KEPT — used by non-CDP code (tab.rs, vertical_tabs.rs, inline_history, terminal_pane)
- Callers excised across 12 files: terminal/view.rs struct fields + render + handler, action.rs ToggleConversationDetailsPanel variant, init.rs keybinding + const, pane_impl.rs toggle button render, ambient_agent/view_impl.rs 3 CDP methods + call sites, pane_group/mod.rs suppress call, workspace/util.rs is_transcript_details_panel_open, workspace/view.rs transcript fields + render + handler + mobile overlay, workspace/view/wasm_view.rs CDP functions, agent_management/view.rs details_panel + selected_item_id; pane_group/mod_tests.rs CDP assertions removed
- `ambient_agent_task_id_for_details_panel` KEPT — used by pane header indicator, workspace routing, agent_icon
- Binary: −0.60 MB real shrink (ConversationDetailsPanel was live via ctx.add_typed_action_view in TerminalView::new() + Workspace::new())
- **KEY LESSON:** handoff's "only used by CDP" claim was wrong for conversation_status_ui + conversation_utils. Always grep before deleting.

**Session 13 deletions done:**
- `ai/predict/` entire directory: `next_command_model.rs` (835 LoC), `predict_am_queries.rs`, `generate_ai_input_suggestions.rs`+tests, `generate_am_query_suggestions.rs`, `prompt_suggestions/mod.rs` + all
- `terminal/view/inline_banner/prompt_suggestions.rs` (381 LoC) — AI-only inline banner
- All callers excised: terminal/input.rs (struct fields, methods, action variants), terminal/view.rs (imports, resolve_prompt_suggestion→false, passive_code_diffs_enabled→false, banner methods deleted), editor/view/mod.rs, pane_group/mod.rs, workspace/action.rs+view.rs+mod.rs, ai/agent_management/view.rs
- `generate_autosuggestion_async`: PartialNextCommandSuggestions+similar_history blocks deleted; `get_reverse_chronological_potential_autosuggestions` inlined; `is_command_valid` → unconditional
- Binary: real shrink (NextCommandModel live via ctx.add_model; PromptSuggestionsView via ctx.add_typed_action_view)

**Session 12 deletions done:**
- `suggested_agent_mode_workflow_modal` + `suggested_rule_modal` + `summarization_cancel_dialog` (live-linked)
- `telemetry_banner` + HideTelemetryBannerPermanently action + should_collect_ai_ugc_telemetry cascade
- `ai/blocklist/usage/` dir (35K LoC total)
- `codebase_index_speedbump_banner` + 4 methods + InlineBannerType variant + settings fields
- `avatar_disc`, `suggestion_chip_view`, `telemetry` orphan modules
- `prompt/prompt_alert.rs` (537 LoC) — live-linked via ctx.add_typed_action_view in 2 places
- `prompt/plan_and_todo_list.rs` (473 LoC) — live-linked; ContextChipKind::AgentPlanAndTodoList removed
- Fixed: pub mod block E0365 visibility bug + stale incremental cache masking compile errors

**Remaining in `ai/blocklist/` (KEEP or deferred):**
- `cli_controller.rs` + `cli.rs` — KEEP (CLISubagentController real functionality)
- `code_block.rs` — KEEP (code rendering in terminal output)
- `handoff/` — cloud agent handoff; used by ambient_agent + remote_server + terminal/input
- `keystroke_render.rs` — KEEP (keyboard shortcut rendering in terminal)
- `permissions.rs` — BlocklistAIPermissions; all callers AI territory; defer
- `persistence.rs` — SerializedBlockListItem load-bearing; PersistedAIInput AI-only; defer
- `prompt.rs` — just `PromptIconButtonTheme` (50 LoC); 1 caller (universal_developer_input.rs)
- `input_stubs` inline in mod.rs — BlocklistAIInputModel stub; needed until spider files cleaned
- `view_util.rs` — KEEP (colors/icons used in terminal UI)
- `request_input.rs`, `response_stream_id.rs`, `session_context.rs` — KEEP (AI session types)

**`ai/predict/` — NEXT large target (session 13):**
- `next_command_model.rs` (36K LoC) — **LIVE-LINKED** via `ctx.add_model(NextCommandModel::new)` in `terminal/input.rs:2288`. 24 refs in input.rs + callers in `editor/view/mod.rs`.
- `generate_ai_input_suggestions.rs` (7.7K) + `generate_am_query_suggestions.rs` — only in server_api.rs; easy 2-file deletion.
- `predict_am_queries.rs` — used in server_api.rs + terminal/input.rs:166,10445.
- `prompt_suggestions/` subdir — const ACCEPT_PROMPT_SUGGESTION_KEYBINDING used in 2 places (inline it); functions `has_pending_code_or_unit_test_prompt_suggestion`, `is_accept_prompt_suggestion_bound_to_*` used in terminal/input.rs + terminal/view.rs.

**Session 13 plan:**
1. Remove `generate_ai_input_suggestions` + `generate_am_query_suggestions` from server_api.rs → delete those 2 files
2. Inline ACCEPT_PROMPT_SUGGESTION_KEYBINDING const → remove `prompt_suggestions/` module dep from init.rs + prompt_suggestions.rs
3. Remove `next_command_model` from terminal/input.rs (24 refs) — biggest win (~36K LoC, live-linked)
4. Remove from editor/view/mod.rs
5. Delete `ai/predict/` entirely

Key note: `is_command_valid`, `is_next_command_enabled` from next_command_model are also imported in terminal/input.rs. Replacing with `false` simplifies excision.

**PREVIOUS STATE (2026-06-01 session 9 final):**
- 3-gate: **0/0/0** (commits `28c4ed8b`, `1f3f26b3`, `2640a60f`)
- Binary: **772.9 MB** (stubs still live — no binary change until stubs deleted)
- Major progress: workspace/view.rs (~1300 LoC removed), terminal/view.rs (~600 LoC removed), many Group A/B/C files cleaned
- Stubs still live; some files re-acquired stub imports due to parallel agent conflicts

**Key remaining stub files for session 10:**
- `pane_group/pane/terminal_pane.rs` — 52 stubs (reverted by agent)
- `ai/agent/conversation.rs` — 50 stubs
- `terminal/input.rs` — 31 stubs (partial, reverted)
- `terminal/view.rs` — 26 stubs (`AIBlock` in rendering + `PendingQueryState` in public API)
- `pane_group/mod.rs` — 19 stubs
- `terminal/view/context_menu.rs` — 1 stub

**SESSION 9 LESSON (critical): Parallel agents cause chaos.**
Running 8+ concurrent agents that all write to overlapping files caused repeated reversions. Each agent's cleanup was undone by another. Use SERIAL agent approach in session 10.

**NEXT: Step 8c (session 10) — serial stub caller excision + delete stubs**

Full session 10 handoff at `/tmp/session10-handoff.md`

Key remaining stub files (51 total, current counts):
- `pane_group/pane/terminal_pane.rs` — 52 stubs (reverted by parallel agent conflict)
- `ai/agent/conversation.rs` — 51 stubs
- `terminal/view.rs` — 41 stubs
- `terminal/input.rs` — 37 stubs
- `pane_group/mod.rs` — 18 stubs
- ~46 smaller files (1-16 stubs each; many are trivial 1-3 line fixes)

Strategy for session 10 (**CRITICAL: SERIAL, NOT PARALLEL**):
1. Tier 1: 4 trivial test files (2 lines each) → commit
2. Tier 2: ~30 Group C files serially → commit batches
3. Tier 3: delete 4 AI-only test files (via removing `#[path]` decls)
4. Tier 4 (spider files — ONE AT A TIME with cargo check between each):
   a. `terminal_pane.rs` → commit; b. `ai/agent/conversation.rs` → commit
   c. `terminal/input.rs` → commit; d. `terminal/view.rs` → commit
   e. `pane_group/mod.rs` → commit
5. Delete 5 stub files + clean mod.rs + remove lib.rs 2 lines → 3-gate → ~10-25 MB drop

**LESSON (session 9):** Full stub deletion requires ALL spider files clean first. Attempted all-at-once deletion produced 109+ errors. Correct approach: excise AI branches from each spider file before deleting stubs. Use Python batch scripts for Groups A/B/C; hand-edit spider files with full context.

49 files still import stub-provided types. Full list via:
```bash
grep -rln "ai::blocklist::block::\|ai::blocklist::\(AIBlock\|AIBlockEvent\|BlocklistAIHistoryModel\|BlocklistAIHistoryEvent\|BlocklistAIContextModel\|BlocklistAIActionModel\|BlocklistAIInputModel\|OrchestrationEvent\|TaskStatusSync\|LocalSharedSession\|StartAgentRequest\|ShellCommandExecutor\|PendingAttachment\|PendingQueryState\|ConversationStatusUpdate\|block_context_from\|descendant_conversation\|history_model::\)" app/src/ --include="*.rs" | grep -v "blocklist/\(block_stubs\|action_stubs\|history_model_stubs\|context_model_stubs\|orchestration_stubs\|mod\)"
```

Order: Group A (pure AI, ~12 files) → Group B (tests, ~10 files) → Group C (smaller shared, ~19 files) → Group D (spider files one at a time):
1. `terminal/input.rs` (~11k LoC) — CLISubagentController, BlocklistAIContextModel, BlocklistAIHistoryModel, etc.
2. `workspace/view.rs` (~20k LoC) — history_model module, BlocklistAIHistoryEvent, PendingQueryState, FORK_PREFIX
3. `terminal/view.rs` (~24k LoC) — largest; AIBlock, all AI model types, CLISubagentView/Controller

After all 49 clean: delete stubs, remove lib.rs BlocklistAIHistoryModel lines, run 3-gate. Large binary reduction expected (~10-25 MB).

Full caller map + excision patterns → `/tmp/phase-i-handoff.md`.

**LESSON (session 8):** Full stub deletion requires ALL spider files clean first. Attempted all-at-once deletion produced 109+ errors. Correct approach: excise AI branches from each spider file before deleting stubs. Use Python batch scripts for Groups A/B/C; hand-edit spider files with full context.

### Sub-task A: Relocate shared non-AI modules — ✅ DONE

All done. New canonical locations in `terminal/view/`:
- `inline_action_icons.rs` ✅ (old file = pub use re-export)
- `inline_action_header.rs` ✅
- `requested_action.rs` ✅
- `requested_script.rs` ✅ (also needed by `terminal/ssh/install_tmux.rs`)
- `keyboard_navigable_buttons.rs` ✅
- `toggleable_items.rs` ✅
- `with_content_item_spacing.rs` ✅ (extracted from block/view_impl.rs)

All non-AI callers updated to canonical paths. Old files in block/ and inline_action/ are pub use re-exports.

**Two items from original list deferred:**
- `secret_redaction.rs` — has AI-type deps (`AIBlockAction`, `AIAgentOutput`, `AIAgentTextSection`) baked in; pure-function part (`find_secrets_in_text*`) can be split out, but requires reading 717 lines to identify the split. Deferred to Phase G pass.
- `numbered_button.rs` — only used by `keyboard_navigable_buttons.rs` (now in terminal/view via `crate::ai::blocklist::block::numbered_button::render_recommended_badge`) and `number_shortcut_buttons.rs` (AI). Must relocate `render_recommended_badge` to terminal/view before block/ can be deleted.

### Sub-task B: Delete inline_action/ directory — ✅ DONE (`e481e765`)

All CodeDiffView callers cleaned first (pre-step):
- Deleted: `passive_code_diff.rs`, `code_diff_pane.rs`, `code_diff_pane_model.rs`
- Removed CodeDiffView from: terminal/view.rs (on_maa_code_diff_generated, open_code_diff, OpenCodeDiff event), workspace/view.rs (open_code_diff method, code_diff_paths, handler, is_code_diff_pane), pane_group/ (all variants + methods), vertical_tabs.rs
- Removed DiffSessionType + register_file + finish_file_registration from code/inline_diff.rs
- Deleted orchestration_config_block.rs + 10 refs from ai_document_view.rs

Then inline_action/ directory deleted (27 files, 14,357 LoC). 3-gate 0/0/0.

**Also fixed in oracle loop:**
- `code_block.rs` imports updated to terminal/view/ canonical locations
- `block_stubs.rs` CodeDiffView/SuggestedUnitTestsView → `Option<()>` stubs

### Sub-task C: Delete block/ directory and block.rs — ✅ DONE (`f048a967`)

Deleted `ai/blocklist/block.rs` (6481 LoC) + `block/` dir (32 files, ~16k LoC). Added:
- `app/src/secret_redaction.rs` — pure fns (find_secrets_in_text*, SECRET_REDACTION_REPLACEMENT_CHARACTER) extracted so non-AI callers survive
- `ai/blocklist/block_stubs.rs` (~800 LoC) providing AIBlock, AIBlockEvent, AIBlockAction, AIBlockResponseRating, CLISubagentView, CLISubagentController, BlocklistAIStatusBar, view_impl, compact_agent_input, number_shortcut_buttons, find, status_bar, cli, model sub-modules — all via `#[path = "block_stubs.rs"] pub mod block;` redirect
- Inlined `render_recommended_badge` into `terminal/view/keyboard_navigable_buttons.rs`
- Updated 6 non-AI callers of secret_redaction to `crate::secret_redaction::*`

**Lesson:** stub approach required ~25 oracle iterations and 800 LoC of stub code. Binary stays flat because `inline_action/` callers are still alive. **Next session should use delete-and-fix instead of delete-and-stub — delete inline_action/ simultaneously and fix errors by removing call sites.**

### Sub-task D: Delete all stub files

**Step 8 (session 8):** Delete all 5 stub files + oracle loop across ~60 callers.
- Delete: `action_stubs.rs`, `orchestration_stubs.rs`, `context_model_stubs.rs`, `history_model_stubs.rs`, `block_stubs.rs`, `block_tests.rs`
- Remove all stub mod+use from `mod.rs`
- Remove `lib.rs` singleton regs: `BlocklistAIHistoryModel` (lines 136 + 1474), `BlocklistAIPermissions`
- See `/tmp/phase-h-handoff.md` for full caller map + recommended excision order

### Sub-task E: Clean up Input struct + TerminalView — ✅ DONE (`92742608`)

Removed `ai_input_model`, `ai_context_model`, `ai_action_model`, `agent_status_view` fields from `Input` struct. All ~300 method-body refs fixed by excision. −2,515 LoC. 19 AI input-mode tests deleted. Binary 776.7 MB (−0.71 MB). 3-gate 0/0/0.

`agent_status_view` creation removed from Input::new(); view.rs cancel_active_conversation_via_status_bar() → no-op; summarization_cancel_dialog_handle() → None; workspace/view.rs ai_context_model().update() → deleted.

Note: `SlashCommandModel`, `TerminalInputMessageBar`, `models/view.rs` still have AI model fields — they compile fine (passed from Input::new() locals). Will be cleaned when stubs are deleted in Sub-task D.

Also delete `permissions.rs` (all AI callers) + stub/delete `persistence.rs` (relocate `SerializedBlockListItem` first).

**Correct Phase F+G order (CRITICAL — two subagents got this wrong):**

```
Phase G-preview FIRST: excise ai_controller from terminal/view.rs
Phase F AFTER: delete block.rs + model layers
```

**Why:** `TerminalView` has `ai_controller: ModelHandle<BlocklistAIController>` as a struct field, with 369 AIBlock/BlocklistAI* refs in `terminal/view.rs` (23,991 lines). Deleting the model layer without excising callers violates the green-always invariant. Same root cause as the original abandoned `strip-cloud` branch.

**Phase G-preview target** — excise from `terminal/view.rs`:
1. Remove `ai_controller: ModelHandle<BlocklistAIController>` struct field + all constructor/update sites
2. Remove `AIBlockModelImpl`/`BlocklistAIController` construction (lines ~2980, ~4767, ~4855, ~4899)
3. Remove `BlocklistAIControllerEvent` + `BlocklistAIActionEvent` + `BlocklistAIContextEvent` subscriptions
4. Remove AIBlock rendering branches (369 sites via cargo check oracle)
5. Simultaneously fix `context_chips/display.rs` (ai_input_model/ai_context_model fields), `pane_group/mod.rs` (14+ BlocklistAIHistoryModel calls), `terminal/input/slash_commands/`, data sources in `terminal/input/`, `tab.rs`, `workspace/view.rs`, etc.

**CRITICAL CASCADE LESSON (2026-05-31 failed attempt):** Do NOT remove `ai_input_model`, `ai_context_model`, `ai_action_model` from the `Input` struct in Phase G-preview. Those 3 fields are used by 50+ method bodies in `terminal/input.rs` and its submodule render files (`classic.rs`, `universal.rs`, `terminal.rs`, `decorations.rs`). Removing them causes 150+ errors that can only be fixed by editing/deleting those render methods — which is Phase F work. **Phase G-preview scope = remove `ai_controller` (and its direct dependents: passive_suggestions_models, cli_subagent_controller, ai_render_context, agent_todos_popup, get_relevant_files_controller). Leave `ai_input_model`/`ai_context_model`/`ai_action_model` in BOTH TerminalView and Input until Phase F.**

**Recast query (verified working — 28 AI-only methods in terminal/view.rs):**
```
(function_item name: (identifier) @name (#match? @name "^(handle_ai_controller_event|handle_legacy_passive_suggestions_event|handle_ai_context_model_event|handle_ai_history_model_event|handle_cli_subagent_controller_event|handle_resume_conversation|handle_usage_footer_toggled|handle_ai_input_model_event|handle_ai_action_model_event|focus_ai_block_if_self_focused|handle_maa_passive_suggestions_event|handle_ai_block_event|active_ai_block|last_ai_block|handle_shell_command_executor_event|handle_start_agent_executor_event|get_ai_notification_summary|maybe_insert_tombstone_for_non_running_shared_ambient_task|on_next_conversation_finished|update_context_blocks_and_exchanges|drop_hidden_passive_ai_blocks|clear_prompt_suggestions|try_clear_prompt_suggestions_banner_code_state|remove_pending_cloud_mode_query_if_exchange_has_renderable_user_query|pending_user_query_conversation_id|remove_pending_user_query_block|stop_local_agent_conversation|apply_cli_agent_footer_visibility)$")) @fn
```
Also delete (second recast pass, 7 methods): `build_agent_todos_popup|handle_agent_todos_popup_event|ai_controller|ai_context_model|ai_input_model|active_conversation_id|active_conversation_task_id`

**Input::new() — only remove these params (keep ai_input_model, ai_context_model in signature):**
`ai_controller`, `ai_action_model`, `cli_subagent_controller`

**TerminalView struct fields to remove (minimal set):**
`ai_controller`, `passive_suggestions_models`, `get_relevant_files_controller`, `ai_render_context`, `conversation_ended_tombstone_view_id`, `conversation_completed_callbacks`, `cli_subagent_views`, `cli_subagent_controller`, `pending_user_query_view_id`, `pending_user_query_kind`, `queued_prompt_callback`, `is_todo_popup_visible`, `agent_todos_popup`, `usage_footer_view_ids`
Keep: `ai_action_model`, `ai_input_model`, `ai_context_model`

**Phase F deletions status:**
- ~~`ai/blocklist/controller/` + `controller.rs`~~ ✅ `202b79e0`
- ~~`ai/blocklist/passive_suggestions/`~~ ✅ `202b79e0`
- ~~`ai/blocklist/action_model/` + `action_model.rs`~~ ✅ `c42b65cb` (stubbed)
- ~~`ai/blocklist/orchestration_events.rs` + `orchestration_topology.rs` + `orchestration_event_streamer.rs` + `orchestration_conversation_links.rs`~~ ✅ `0576df88` (stubbed)
- ~~`ai/blocklist/task_status_sync_model.rs`~~ ✅ `0576df88` (stubbed)
- ~~`ai/blocklist/local_shared_session_link_model.rs`~~ ✅ `0576df88` (stubbed)
- ~~`ai/blocklist/context_model.rs`~~ ✅ `b3ae3d79` (stubbed → context_model_stubs.rs)
- ~~`ai/blocklist/input_model.rs`~~ ✅ `97a134ef` (stubbed → input_model_stubs.rs)
- ~~`ai/blocklist/history_model.rs` + `history_model/`~~ ✅ `125c0f72` (stubbed → history_model_stubs.rs, #[path] redirect)
- **`ai/blocklist/permissions.rs`** — still alive; all callers are AI territory. Delete with Phase G pass.
- `persistence.rs` DEFERRED — `SerializedBlockListItem` load-bearing for session restore. Relocate before delete.
- ~~`ai/blocklist/block.rs` + `block/`~~ ✅ `f048a967` — deleted, replaced by `block_stubs.rs`

**Phase F sub-task C lessons (2026-06-01):**
- **Stub approach cost**: block/ deletion required ~25 oracle passes and ~800 LoC of stubs because `inline_action/` (still alive) imported deeply from block/. Binary stays flat.
- **Delete-and-fix beats delete-and-stub**: if only consumers of a deleted module are themselves AI-only and scheduled for deletion, delete them simultaneously. Fix errors by removing call sites, not by adding stubs.
- **#[path] redirect** works well for single-module swap: `#[path = "block_stubs.rs"] pub mod block;` preserved all `crate::ai::blocklist::block::*` import paths without touching callers.
- **Empty::new().finish() in sub-modules**: each inline `mod {}` block needs its own `use warpui::Element;` — parent imports don't scope in.
- **Generic ctx params** avoid type mismatch: `pub fn method<C>(&self, _: &mut C)` accepts ModelContext/ViewContext interchangeably.

**NEXT SESSION — Phase G continued: fix direct field access, then delete struct fields**

Before deleting `ai_context_model`/`ai_input_model`/`ai_action_model` from `TerminalView` + `Input`, fix these direct field accesses:
1. `terminal/view/pane_impl.rs:210,644` — `self.ai_context_model.as_ref(app)` → singleton call on `BlocklistAIContextModel`
2. `terminal/view/context_menu.rs:158` — `self.ai_action_model` direct access → no-op / singleton
3. `workspace/view.rs`, `code_review_view.rs`, `ai_document_view.rs` callers of `terminal_view.ai_context_model()` getter
4. `terminal/input/` data sources calling `Input.ai_context_model()`

After those are fixed:
5. Remove `ai_context_model`, `ai_input_model`, `ai_action_model` fields from `TerminalView` + `Input` structs
6. `rm -rf app/src/ai/blocklist/inline_action/` (40+ files, all AI rendering)
7. Delete `block_stubs.rs` + all `*_stubs.rs` in `ai/blocklist/`
8. Remove AI import blocks from `terminal/view.rs`, `workspace/view.rs`, `terminal/input.rs`, `pane_group/mod.rs`
9. Run cargo check → fix each error by **deleting the call site**, not stubbing

This sequence unblocks the real LoC reduction + binary shrink.

**KEEP in blocklist/:** `prompt/`, `view_util.rs`, `keystroke_render.rs`, `code_block.rs`. KEEP `ai/mcp/`.

**KEEP terminal/input/:** `inline_menu/`, `message_bar/`, `inline_history/`, `cloud_mode_v2_history_menu.rs` — shared terminal UI infrastructure used by 28+ non-AI modules. NOT AI-only.

---

### shared_session strip — original plan (reference)

**Goal**: remove Warp's terminal session-sharing (share live session over Warp cloud; join/view via link). NOT a generic AI capability — vendor CLIs never touch it.

**Two homes + core type**:
- `terminal/shared_session/` (dir, ~14.8k LoC) — engine: manager, network, sharer, viewer, presence_manager, permissions_manager, participant_avatar_view, render_util, replay_agent_conversations, role_change_modal, share_modal, shared_handlers, selections, settings, ai_agent.
- `terminal/view/shared_session/` (dir, ~6.7k LoC) — view layer: adapter, view_impl, conversation_ended_tombstone_view, cloud_conversation_continuation, sharer/viewer view. Depends on dir #1.
- **`SharedSessionStatus`** (in `terminal/shared_session/mod.rs:99`) — the sharer/viewer/reader/executor permission state ON `terminal_model`. ~11 predicates (`is_viewer`/`is_sharer`/`is_reader`/`is_executor`/`is_active_sharer`/…). Read at **111 sites**, heaviest in `terminal/input.rs` + `terminal/view.rs`.

**Done**: ✅ commit 1 `144639a4` — AI agent_sdk + `warp_cli::share` decouple (ShareSessionError, should_share, wait_for_session_shared, add_share_requests, EstablishedSharedSession event, write_session_joined, --share flag). 3-gate green, 0 warnings. terminal/shared_session still mounted. Binary 911.6 → 911.3 MB (`521aad4b`, real −274 KiB — sharing path was live-linked).

**EXECUTION METHOD — use this, do NOT hand-Edit (proven over 3 attempts)**: the remaining strip is ~200-400 edits / ~28 source files + ~15 test files. The `Edit`/`Write` tools auto-inject a multi-KB diagnostics wall *per call* → ~200 edits blows the context budget every time (3 interactive attempts walled out at ~file 10 and were reset). **Fix: drive ALL edits through batched `Bash`/`python3` heredocs** (exact `str.replace` with not-found asserts) — a python batch of N replacements returns one clean tool result, NO diagnostics wall. Setup (rm both dirs + `cat >` the stub + `perl -0pi` the `mod shared_session;` decls) is one Bash call. Run `cargo check -p warp 2>&1 > /tmp/cc.txt` only at batch boundaries; read sites via `sed -n`/`grep` (cheap, no wall). This makes it tractable — but it is still a **dedicated fresh-session job that must open with this method**, not a tail-end task. The 28-file error list + the exact auth/auth_manager/settings replacement strings are reproducible (all 3 landed cleanly via one python batch). Order: setup → known-exact batch (auth/mod, auth_manager, settings/init) → IsSharedSessionCreator full removal (field unread-warns, so must delete the NewTerminalOptions field + child_agent/terminal_pane/docker_sandbox/ambient_agent threading) → terminal_model data-plane removal → SharedSessionActionSource + share-UI methods + call sites (view.rs 68 / terminal_manager 38 / pane_group 23 — the giants) → tab/alt_screen/block_list/input/drive/quit_warning → lib.rs mounts → 4 feature flags → ALL test files → 3-gate green.

**Strategy (advisor-confirmed): COLLAPSE, not full removal — ONE atomic commit** (0-warning gate chains: removing a consumer orphans a machinery method → warning → cascade; can't yield green sub-points). Like autoupdate (one big commit).
1. Collapse `SharedSessionStatus` → single `NotShared` variant; every predicate returns `false`; drop `Role` field (sheds `session_sharing_protocol` dep on the enum); `as_keymap_context()` → always `"SharedSessionStatus_NotShared"`. Definitionally correct for a no-sharing build. 111 read-sites keep compiling (dead branches) — **full read-site removal DEFERRED, flag honestly**.
2. **Flatten** dir → `terminal/shared_session.rs` (~40-line stub: just the collapsed enum + predicates). Import paths stay `crate::terminal::shared_session::SharedSessionStatus` → **zero import-rewrite churn** (~35 files). Dir 21.5k→40 LoC IS the strip.
3. **Fully remove** `IsSharedSessionCreator`/`SharedSessionSource` (only `::No` survives post-commit-1) + `NewTerminalOptions.is_shared_session_creator` field + threading (~10 all-No sites).
4. Delete both dirs' machinery + `terminal/view/shared_session/` + `Event::{EstablishedSharedSession,FailedToShareSession}` in `terminal/view.rs`.
5. Remove the share-action surface: `SharedSessionActionSource` enum + methods `open_share_session_modal`/`stop_sharing_session`/`copy_shared_session_link`/`copy_session_link` + their callers in view.rs, view/pane_impl.rs, view/action.rs, view/init.rs, view/use_agent_footer/mod.rs, workspace/view.rs, local_tty/terminal_manager.rs. Also `SharedSessionScrollbackType` (machinery-only after this).
6. Decouple: auth/mod.rs + auth_manager.rs (Manager stop_all/rejoin_all + `num_shared_sessions` warning), session_management.rs (`shared_session_status` field + `num_shared_sessions` fn), pane_group (viewer::TerminalManager downcast, share_modal, role_change_modal, `number_of_shared_sessions`, ParticipantAvatarParams, presence), tab.rs (indicator color), alt_screen, block_list_element, terminal/input.rs (PresenceManager), settings/init.rs (SharedSessionSettings), drive/sharing, quit_warning (counting).
7. lib.rs: drop `Manager` + `SessionPermissionsManager` mounts (KEEP `LocalSharedSessionLinkModel` — couples to `session_sharing_protocol`, deferred AI-panel pass).
8. Drop `FeatureFlag::{CreatingSharedSessions,ViewingSharedSessions,SharedSessionWriteToLongRunningCommands,AgentSharedSessions}`.

**RISK SPOT**: `local_tty/terminal_manager.rs` setters (`set_shared_session_status(ActiveSharer/NotShared)`) — confirm no local-session-lifecycle side-effect lived only inside a sharing branch before cutting. Can't headless-test GUI.

**Keep alive**: `session_sharing_protocol` crate (deferred AI-panel submod `ai/blocklist/controller/shared_session.rs` + `local_shared_session_link_model.rs` still use it); `ai/blocklist/controller/shared_session.rs` (871 LoC, separate AI-panel viewing, no terminal-dir coupling).

**Honest commit/plan framing**: "machinery removed, status enum collapsed to vestigial NotShared stub, 111 read-sites neutered to always-local; full read-site removal deferred." NOT "shared_session removed."

### Cleanup TODO (from onboarding pass — non-blocking, build+tests green)
- ✅ Onboarding action stubs + dead variants/enums + helper methods removed (`d444c2b6`). `ImportSettings` was NOT a stub to delete — it was a mis-stubbed local feature, restored (`e9a3dc2e`).
- **Remaining 79 warnings — DEFERRED to subsystem passes (not blindly swept).** They are orphaned unused imports + dead fns left by the onboarding `AgentOnboardingEvent` handler removal, but they belong to other subsystems and are cleanest removed *with* those subsystems (advisor guidance — avoids feature-gate false-positives + keeps per-pass attribution):
  - Login pass: `login_slide.rs` (`VISUAL_IMAGE_PATHS`, `resolve_visual_path`, `is_auth_token_input_visible`/`new`/`handle_auth_manager_event`, `LoginSlideSource` variants), `paste_auth_token_modal::new`, root_view imports `LoginSlide*`/`PasteAuthTokenModalEvent`.
  - AI pass: `LLMPreferences`/`refresh_*_models`, `HistoryEvent`, `AISettings`, `EntrypointType`/`StaticQueryType`.
  - Themes pass: `ThemeKind`, `WarpThemeConfig`, `ThemeSettings`.
  - Cloud/billing pass: `CloudPreferencesSyncer*`, `PricingInfoModelEvent`, `build_plan_yearly_price_cents`, `upgrade_url`.
  - Experiments pass: `is_free_user_no_ai_experiment_active`.
  - Teams pass: `TeamUpdateManager`, `UserWorkspaces*`, `TabSettings`.
  - Generic orphans (could sweep anytime, low value): root_view `ClipboardContent`/`report_if_error`; terminal/view `TerminalKeybindings`.
  - Upstream style (NOT ours): slash_commands `binding if guard => {}` unused-var cluster (~40 warnings) in `terminal/input/slash_commands/mod.rs`.
  - callout_bubble (`callout_title_color`/`callout_body_color`/`CalloutArrowPosition::{Start,End}`): onboarding-callout leftover but `callout_bubble` is shared — verify other users before trimming.
- 2 `let _ = (...)` placeholders: `workspace/view.rs` (insert-drive caller), session-config `(has_worktree, has_params)`.
- `crates/onboarding` still present (only `auth/login_slide.rs` uses `onboarding::slides::{layout,slide_content}` + `OnboardingIntention` + `AI_FEATURES`/`WARP_DRIVE_FEATURES`). Delete crate in the **login pass** after relocating those ~470 LoC into `auth/`.

### Techniques & lessons (reusable for remaining passes)
- **cargo-check-driven loop**: delete the def (enum variant / module / method), let `cargo check` enumerate every break site, fix in batches, repeat to 0. Worked for welcome + onboarding.
- **sed range-delete pitfall**: deleting `[fn_start, next_fn-1]` by a grep'd fn-map can silently eat a *free fn* or an `impl Foo {` opener sitting between two methods → "unexpected closing delimiter" brace imbalance far away. Before range-deleting, check what's between adjacent methods (free fns, impl boundaries). Recovery: find the orphaned `impl {` / free fn via `git show HEAD:<file>` and re-insert. (This pass: lost `fork_label_for_query` + an `impl TerminalView {` opener.)
- **LSP works on the worktree now** (session rooted here). But injected diagnostics LAG edits — they show stale errors at old line numbers. Trust `cargo check`, not the diagnostic stream.
- **Name-based "useless" guesses need code-reading**: `welcome_palette` was the new-tab landing surface; the "Welcome to Warp/Get started/Log in" screen was the *onboarding intro slide*, not the plain login gate. Read before deleting.
- **no-op-stub-then-clean**: to reach green fast on a deeply-woven match arm, stub `Variant(_) => {}` + flag it, rather than cascading a variant removal through a 767-arm enum mid-pass. Clean in a follow-up.
- **no-op-stub can hide a regression**: a stub silences the compiler but also silently kills a still-reachable feature. `ImportSettings=>{}` looked like onboarding dead code but the "Import External Settings" command is gated on config-detection (`HAS_SETTINGS_TO_IMPORT_FLAG`, `local_fs`), independent of onboarding — stubbing it left a dead palette command. **Before deleting a stub, trace its trigger on the baseline (`git show 0b737e22:` / `git grep <Action> <baseline>`) — if anything other than the removed feature dispatches it, it's a regression to restore, not dead code.** Helper-method names lie too (`add_settings_import_block` set `block_onboarding_active` but wasn't onboarding-only).
- **feature-gate trap for dead-code sweeps**: `cargo check` (default) flags imports/fns unused for *that* build; an item used only under an off-by-default feature (e.g. `local_fs`, wasm) is still flagged. Removing it breaks that feature build. Verify restored `local_fs` code with `cargo check -p warp --features local_fs`. Don't blindly sweep default-unused imports that may be feature-gated-used.
- **`recast`** is available for repeated multi-file identical edits; **LSP `findReferences`** for complete call-site maps.
- **macro call-site sweep with non-greedy regex**: `(?s)NAME!\(.*?\);` reliably matches a whole macro invocation (single- or multi-line). Counted match-count against `\rg -c 'NAME!'` to confirm 1-for-1 — equal counts mean no inner `);` ate the regex early. **Trap**: `\b` matches starting at the macro name, so a `crate::NAME!(...)` prefix leaves a dangling `crate::` after the deletion. Follow-up pattern `(?m)^(\s*)crate::[ \t]*\n` and `(?m)^(\s*)crate::[ \t]+\}` cleaned the orphans. Anchor the leading path next time: `(?s)(?:[\w:]+::)?NAME!\(.*?\);`.
- **cargo fix on `pub use` lines and cfg-test re-exports**: `cargo fix` *will not* touch `pub use` (treated as intentional API) but *will* drop a regular `pub use foo::{X, Y}` brace entry whose Y is only used by `#[cfg(test)]` code. Restoring it (e.g. `pub use permissions::CommandExecutionPermissionAllowedReason`) is mechanical once `cargo check --tests` flags the unresolved import. Bake a `--tests` check into the post-fix re-verify, not just default.
- **events.rs payload-struct prune needs intra-file ref tracking**: a script that counts external refs only (`\rg name --type rust | grep -v <file>`) misses the case where item X is a field type in item Y that's also in the file. Items used only by other items in the same file appear dead but are alive transitively. Fix: cargo-check-driven cascade (delete → fix breaks → repeat) or build the intra-file ref graph first.
- **gate matrix: 3 runs, not 4**: `cargo check -p warp` + `--tests` + `--features local_fs,gui` (combined). Features compose for warp, so the combined run catches both gui- and local_fs-gated regressions in one pass.

### Known upstream gaps (found while stripping — not our bugs, out of scope)
- **Alacritty importer ignores modern config layout.** `app/src/settings/import/alacritty_parser.rs` `AlacrittyConfig` deserializes **top-level** `import` + `colors` only. Alacritty ≥0.13 moved `import` (and several keys) under `[general]`, so a config using `[general].import = [...]` parses to `import: None` → theme never resolved → `is_valid()` false → 0 configs → `HAS_SETTINGS_TO_IMPORT_FLAG` never set → "Import External Settings" command stays hidden. Verified via production parse path (`Config::create_from_external_configs::<AlacrittyConfig>`): `CONFIGS_COUNT=0` with `[general].import`, `=1` with top-level `[colors.*]`. Fix (if ever wanted, beyond strip scope): add a `general: Option<{ import: Vec<String> }>` field + merge it into top-level. Workaround for testing the import UI: inline `[colors.*]` at top level.

---

## KEYSTONE: ai/agent/ core deletion plan (mapped 2026-06-06, session 57)

Two investigator passes mapped the full boundary. `AIConversation`/`AIAgentOutput`/`Task` have **ZERO production constructors** (all data flows from dead conversation.rs / api deserialization). The entire `ai/agent/` core + its blocklist-rendering + AIBlock-UI + persistence consumers are vestigial.

**KEEP (generic, live external consumers):**
- `ai/agent/conversation_types.rs` — AIConversationId/ConversationStatus/StatusColorStyle/ServerConversationToken (depends only on icons.rs). Used by CLI session status, search UI, persistence schema.
- `ai/agent/icons.rs` (58 LoC) — failed/in_progress/succeeded/yellow_stop/gray_stop icons; LIVE consumers: env_var_collection_block.rs, init_project/lsp_server_selector.rs, init_project/mod.rs.
- blocklist GENERIC (70%): code_block.rs, keystroke_render.rs, prompt.rs/prompt/, view_util.rs, input_config.rs (InputConfig/InputType shell-vs-AI toggle), mod.rs, `SerializedBlockListItem` (persistence.rs:464 — command-block session restore, used by pane_group/persistence).

**DELETE (dead AI core):** everything else in ai/agent/ — conversation.rs + mod.rs (the giant) + api/ + task.rs + task_store.rs + comment.rs + linearization.rs + todos/ + redaction.rs + telemetry.rs + util.rs + conversation_yaml.rs + all *_tests.rs.

**CASCADE consumers (delete with it):** blocklist AI-only files (persistence.rs PersistedAIInput/PersistedAIInputType/PersistedAIAgentActionType/AIQueryHistoryOutputStatus, request_input.rs, response_stream_id.rs, queued_query.rs, handoff/, block_tests.rs); persistence agent cluster (agent.rs read/upsert/delete_agent_conversations + SqliteData.multi_agent_conversations field mod.rs:183 + ModelEvent::Update/DeleteMultiAgentConversation{,s} mod.rs:303/308 + sqlite.rs:657-670 handlers + crates/persistence model.rs:870 AgentConversation/990 AgentConversationData + schema.rs:11 agent_conversations table — NOTE writer emit is conversation.rs:2812, dies with the struct).

**VERIFY-DON'T-TRUST (investigator conflict):** `ai/blocklist/cli_controller.rs` (CLISubagentController) — investigator says dead, but AGENTS.md session-10/11 history says it's live for the KEPT CLI-agent rich-input/detection feature. CHECK callers before deleting.

### Cut order (each a green 3-gate commit):
- **STEP 1 — AIBlock context-menu + fork/rewind UI surface.** Files: terminal/view.rs, terminal/view/action.rs, terminal/view/context_menu.rs, workspace/action.rs, workspace/view.rs, workspace/global_actions.rs, workspace/mod.rs, search/command_palette/view.rs (:811), terminal/input.rs (:2904). Delete: ContextMenuAction AI variants (CopyAIBlock/Query/Output/Conversation, CopyAgentCommand/GitBranch, SavePromptAsAgentModeWorkflow, ForkAIConversation/FromBlock/FromExactExchange) + Debug arms + handlers (view.rs ~14870-14936); TerminalAction OpenAIBlockOverflowMenu/RewindAIConversation/ExecuteRewindAIConversation (action.rs:197/204/212) + handlers + accessibility (view.rs ~16093/16247); ContextMenuType::AIBlockOverflowMenu (view.rs:1733 + arms 1769/1789/1810/17024); context_menu.rs 6 methods (ai_block_copying_menu_items, conversation_text, copy_conversation_text, fork_ai_conversation, create_copy_debugging_menu_item, open_ai_block_overflow_context_menu — KEEP generic show_context_menu); view.rs show_rewind_confirmation_dialog/rewind_ai_conversation/fork_label_for_query/copy_conversation_text; workspace ForkAIConversation/ExecuteRewindAIConversation/ShowRewindConfirmationDialog actions + global_actions fork_ai_conversation + ForkAIConversationParams/ForkFromExchange/ForkedConversationDestination + the command-palette/input dispatchers. (All handlers are no-op or dead-dispatch — investigator-confirmed.)
- **STEP 2 — AIBlock inline rendering refs.** Remaining fork/rewind/AIBlock-render call sites in blocklist rendering (rich_content / block_list_element).
- **STEP 3 — blocklist AI-only types.** persistence.rs PersistedAI* cluster (keep SerializedBlockListItem), request_input.rs, response_stream_id.rs, queued_query.rs, handoff/, block_tests.rs + mod.rs decls/re-exports. (Verify cli_controller first.)
- **STEP 4 — ai/agent/ core + persistence agent cluster.** Delete the core (keep conversation_types.rs + icons.rs), the persistence agent cluster, repoint any AIConversationId/ConversationStatus imports that went through ai::agent::conversation:: re-exports → conversation_types.


### Step 1 refinement (attempted 2026-06-06, reverted to green — finding)
Removing the AIBlock TerminalAction variants (action.rs OpenAIBlockAttachedBlocksMenu/OpenAIBlockOverflowMenu/RewindAIConversation/ExecuteRewindAIConversation/ExecuteRewindFromInlineMenu) is clean for the menu/accessibility/handler arms in terminal/view.rs, BUT the three **rewind** variants entangle with a separate **AI-conversation rewind subsystem** (~11 files): `terminal/input/rewind/{mod,view}.rs`, `terminal/input/suggestions_mode_model.rs` (rewind mode + rewind_conversation_id), `terminal/input/inline_menu/mod.rs`, `terminal/input/slash_commands/mod.rs` (the `/rewind` slash command), `terminal/input.rs` (RewindMenuEvent::AcceptedRewindPoint dispatch), workspace/view.rs + action.rs (ExecuteRewindAIConversation handler + ShowRewindConfirmationDialog), server/telemetry/events.rs (AgentModeRewindEntrypoint). So **split Step 1**: 
- **Step 1a — rewind feature**: delete the `terminal/input/rewind/` dir + `/rewind` slash command + RewindMenu wiring in suggestions_mode_model/inline_menu/input.rs + RewindAIConversation/ExecuteRewindAIConversation/ExecuteRewindFromInlineMenu actions + their handlers + ShowRewindConfirmationDialog workspace action + show_rewind_confirmation_dialog/rewind_ai_conversation view methods + AgentModeRewindEntrypoint telemetry. (AI rewind = dead, no AIConversations exist.)
- **Step 1b — AIBlock fork/copy menu**: OpenAIBlockAttachedBlocksMenu/OpenAIBlockOverflowMenu actions + ContextMenuAction AI variants (CopyAIBlock*/CopyAgent*/SavePromptAsAgentModeWorkflow/ForkAIConversation*) + ContextMenuType::AIBlockOverflowMenu + context_menu.rs AIBlock methods + workspace fork action + ForkAIConversationParams cluster + command-palette/input fork dispatchers.
Both before Step 2 (blocklist rendering) and Step 4 (ai/agent/ core). The TerminalAction-variant removal itself only broke 2 dispatch sites (input.rs rewind-menu accept + workspace rewind dispatch) — confirming the cascade is the rewind subsystem, not a wide match-arm fan-out.

### Step 1a DONE (2026-06-06, `3dfcd8e4`, −0.87 MB)
AI-conversation rewind feature fully removed (rewind/ dir + RewindMenu inline-menu wiring + /rewind slash command + TerminalAction/WorkspaceAction rewind actions + RewindConfirmationDialog TypedActionView + AgentModeRewindEntrypoint telemetry). 3-gate 0/0/0 + warp_cli clean. Lesson: the rewind cut touched 11 files but only broke 2 dispatchers at the action-enum layer; the bulk was the self-contained rewind/ dir + the confirmation-dialog cluster. **NEXT = Step 1b** (AIBlock fork/copy context menu): OpenAIBlockAttachedBlocksMenu/OpenAIBlockOverflowMenu TerminalActions + ContextMenuAction AI variants (CopyAIBlock*/CopyAgent*/SavePromptAsAgentModeWorkflow/ForkAIConversation*) + ContextMenuType::AIBlockOverflowMenu + context_menu.rs AIBlock methods (ai_block_copying_menu_items/conversation_text/copy_conversation_text/fork_ai_conversation/create_copy_debugging_menu_item/open_ai_block_overflow_context_menu — and open_ai_block_attached_context_menu) + workspace fork action + global_actions fork_ai_conversation + ForkAIConversationParams/ForkFromExchange/ForkedConversationDestination + UserQueryMenuAction::ForkFrom + the user-query (fork) menu + command-palette/input fork dispatchers.

### Step 1b-i DONE (2026-06-06, `a56290b3`, −0.05 MB, warnings 93→215)
AIBlock context menu removed (ContextMenuAction AI variants + OpenAIBlock TerminalActions + ContextMenuType AI variants + context_menu.rs AIBlock methods). Nearly flat binary; the removal's main effect was exposing ~122 ai/agent-core items as dead (they were fed only by the copy/fork menu's format_for_copy/rendered_lines/etc.) — confirms the core is now almost fully disconnected, deleted at Step 4. **NEXT = Step 1b-ii** (fork inline-menu feature, ~40 refs, analogous to the rewind subsystem): UserQueryMenuAction::ForkFrom + InlineMenuType::UserQueryMenu + suggestions_mode is_user_query_menu/user_query_conversation_id + user_query_menu_view field + UserQueryMenuView (terminal/input/user_query_menu or similar) + open_user_query_menu + the /fork slash command if any + workspace fork_ai_conversation (view.rs:8465) + global_actions fork_ai_conversation (:207) + ForkAIConversationParams/ForkFromExchange/ForkedConversationDestination + command-palette (view.rs:811) + input (input.rs ~2904) fork dispatchers. Then Steps 2 (blocklist AI rendering) → 3 (blocklist AI types) → 4 (ai/agent core + persistence agent cluster).

### Step 1b-ii DONE (2026-06-06, `bcfe6c9a`, −0.82 MB) — Step 1b COMPLETE
Fork feature removed (user_query/ dir + UserQueryMenu wiring + /fork+/fork-from + WorkspaceAction fork/continue-locally/insert-fork + command-palette ForkConversation + ConversationAction::Fork fork-button overlay). 3-gate 0/0/0 + warp_cli clean. Binary 706.1 MB. **Step 1 (AIBlock UI surface) fully complete.** TRAP found: slash commands ARE collected in a registry list (commands.rs build_* `commands.extend([...])`) — `FORK.clone()` at :565 — grep `commands::X` misses bare `X.clone()` in the list, so deleting a StaticCommand needs the list entry removed too (cargo catches it). **NEXT = Step 2** (blocklist AI rendering: the AIBlock render path in blocklist/ + terminal/view that consumes AIAgentOutput/AIAgentExchange — rich_content/block_list_element AI branches), then Step 3 (blocklist AI types: persistence.rs PersistedAIInput cluster, request_input/response_stream_id/queued_query/handoff/block_tests — VERIFY cli_controller live first), then Step 4 (ai/agent core + persistence agent cluster — the big shrink, clears the ~120 dead-core warnings).

### Step 2 MAPPED (2026-06-06, investigator) — surgical + isolated, ready to execute
The AIBlock rendering path is a THIN dead layer over generic rich-content rendering. AIBlockMetadata has ZERO production constructors; all readers are dead predicates. Removing it does NOT touch generic command-block/image/link/table/SSH/plugin/agent-view rich-content rendering. **DELETE:**
- `app/src/terminal/view/rich_content.rs`: `AIBlockMetadata` struct (39-42); `RichContentMetadata::AIBlock` (207) + `AIOnboardingBlock` (208-211) enum variants; `RichContent::is_ai_block()` (139) + `ai_block_metadata()` (183-187) methods.
- `app/src/terminal/view.rs`: `BlocklistAIRenderContext::context_color_for_rich_content` AIBlock/AIOnboarding match arms (1833-1842); the empty `RichContentMetadata::AIBlock` arm (~11776); `is_exchange_in_active_conversation()` (1785-1789) + `exchange_ids` field (1766); `scroll_to_exchange()` (13147-13172).
- `app/src/terminal/block_list_element.rs`: the `RichContentMetadata::AIBlock` dead branch (~1553) + the ai_render_context color-stripe (`draw_flag_pole`) branch in the RichContent paint arm (~3840-3865) IF it only fires for AIBlock (verify it's not used by other context colors).
- `app/src/terminal/input.rs`: `InputEvent::ScrollToExchange { exchange_id }` (826) + import (122) — dead (never dispatched once scroll_to_exchange gone).
**KEEP:** RichContent wrapper + RichContentMetadata enum structure + all non-AI variants (UsageFooter/InitStep/EnvVarCollection/Ssh*/Warpify*/AgentViewEntry/InlineAgentViewHeader/*ZeroState/PluginInstructions) + insert_rich_content + the generic paint/layout dispatch. **CRITICAL: `AIAgentExchangeId` (ai/agent/mod.rs:2475) MUST SURVIVE** — still used by `blocklist/persistence.rs` PersistedAIInput (35); it's a light ID like AIConversationId. Delete it only in Step 4 with the persistence cluster. Step 2 is cleanly separable from Step 3. Est. shrink ~50-150 KB (thin dead layer; real shrink is Step 4). VERIFY before cut: that `context_color_for_rich_content`'s only callers/colors are AIBlock (the flag-pole stripe) — if other rich content uses a context color, keep that path.

### Step 2 DONE (2026-06-06, `807fe7b3`, −0.02 MB) — AIBlock rendering removed
Surgical as mapped: AIBlockMetadata + RichContentMetadata::AIBlock/AIOnboardingBlock + context_color_for_rich_content + scroll_to_exchange + ScrollToExchange InputEvent, all dead. Generic rich-content rendering intact. AIAgentExchangeId KEPT (blocklist/persistence). 3-gate 0/0/0. **NEXT = Step 3** (blocklist AI-only types): persistence.rs PersistedAIInput/PersistedAIInputType/PersistedAIAgentActionType/AIQueryHistoryOutputStatus (keep SerializedBlockListItem), request_input.rs, response_stream_id.rs, queued_query.rs, handoff/ (touched_repos uses AIConversation), block_tests.rs + mod.rs decls/re-exports. **VERIFY cli_controller.rs first** — AGENTS.md says CLISubagentController is live for the KEPT CLI-agent rich-input feature; do NOT delete it without confirming callers. Then Step 4 (ai/agent core + persistence agent cluster — the big shrink; AIAgentExchangeId dies here).

### Step 3 REASSESSED (2026-06-06) — mostly NOT separable; folds into Step 4
Investigated the "blocklist AI types". Findings overturn the plan's clean Step-3/4 split:
- **`cli_controller.rs` (CLISubagentController) is LIVE** — constructed at input.rs:1706, used by universal_developer_input/slash_commands data_source/interaction_mode/serialized_block for CLI-agent long-running-command control. **KEEP** (AGENTS.md was right; the Step-2 investigator was wrong).
- **`handoff/` is LIVE** — input.rs cloud-handoff compose (touched_repos: pick_handoff_overlap_env/resolve_repo_for_path/sort_environments_by_recency + HandoffLaunchAttachments) + remote_server handoff_snapshot (derive_touched_workspace) + server_model (upload_result_to_proto). **KEEP**.
- **`RequestInput` + `ResponseStreamId`** — used ONLY by the dead core (ai/agent/conversation.rs). Delete WITH Step 4, not before.
- **`PersistedAIInput` / `AIQueryHistoryOutputStatus`** — feed a separate **AI-query-history** subsystem: persistence/mod.rs `ai_queries: Vec<PersistedAIInput>` + persistence/block_list.rs (TryFrom AIQuery/NewAIQuery + ai_queries sqlite) + search/command_search/ai_queries/ (search past AI queries UI). This is its own removal cluster (persistence table + command-search page), separable from the core — flag as **Step 3.5 (AI query history)**.
- **DONE now:** deleted orphan queued_query.rs + block_tests.rs (not compiled).
**Revised order:** Step 3.5 (AI-query-history: ai_queries persistence + command_search/ai_queries) can go independently. Then **Step 4** (ai/agent core) takes RequestInput/ResponseStreamId/PersistedAIInput(if not already)/AIAgentExchangeId/the persistence agent cluster with it — the big shrink. Step 4 is the keystone; everything else is now either LIVE-keep or core-coupled.

### Step 3.5 DONE (2026-06-06, `ea27b66b`, −1.83 MB) — AI-query-history removed
Surgical as mapped. Deleted the command-search ai_queries page + data source + SuggestionType::AIQuery + PersistedAIInput cluster + ai_queries persistence/ModelEvent/load-chain. Kept SerializedBlockListItem + SuggestionType::ShellCommand + sqlite ai_queries table def (DB compat). Real −1.83 MB shrink (search-mixer-linked SearchItem rendering). 3-gate 0/0/0. **ONLY Step 4 remains** = the ai/agent core keystone: delete conversation.rs + mod.rs (the ~17K-LoC AIAgentOutput/Action/Text/Task tangle) + api/ + task/comment/linearization/todos + RequestInput/ResponseStreamId (blocklist) + the persistence agent cluster (agent.rs/AgentConversation/multi_agent_conversations field+sqlite+ModelEvent UpdateMultiAgentConversation/DeleteAIConversation) + AIAgentExchangeId (now ONLY blocklist/persistence.rs structs use it — verify after core gone). KEEP conversation_types.rs (AIConversationId/ConversationStatus/StatusColorStyle/ServerConversationToken) + icons.rs. This is the big one (real shrink + clears ~120 dead-core warnings). Needs its own careful pass — possibly an investigator map of the core's internal module graph + remaining external AIConversationId/ConversationStatus consumers (CLI session status, search, persistence schema, terminal model) that must survive.

### Step 4 MAPPED (2026-06-06) — it's an EXTRACT-then-DELETE, NOT a clean deletion. The hard one.
Investigator + my verification: the `ai/agent/` core is NOT a dead island. It DEFINES the data-model types that **kept, live features** consume — so those types must be EXTRACTED to surviving modules before the core is deleted. Live external consumers (verified live files, not dead/test):
- `task::TaskId` (×9) — cli_controller.rs (LIVE CLI-agent), terminal/model/block*.rs (LIVE terminal blocks), persistence schema.
- `AIAgentActionId` (×6) — editor_management.rs, terminal/model/block.rs, code_review.
- `comment::CodeReview` (×3), `DiffSetHunk`(×4)/`DiffBase`/`CurrentHead` — code_review/ (KEPT generic code-review feature).
- `AgentReviewCommentBatch` (×3) — terminal/cli_agent.rs, workspace/view/right_panel.rs (LIVE).
- `AIAgentInput` (+ ::ActionResult/::UserQuery), `AIAgentActionResultType` (+ many variants), `redaction::redact_secrets`, `ProgrammingLanguage`, `MessageId`, `ImageContext`, `UserQueryMode`, `AIAgentPtyWriteMode`, `AIAgentContext`, `todos::AIAgentTodoList`, `linearization::compute_*`, `util::parse_markdown_into_text_and_code_sections`, `api::convert_conversation::`/`convert_from::`, `AIAgentTextSection`/`AgentOutputTableRendering`/`AgentOutputImageLayout`, `SubagentType`, `ReceivedMessageDisplay`, `InvokeSkillUserQuery` — consumed across terminal/view.rs, terminal/model/block*, code_review, cli_agent, editor_management, lib.rs.
**Implication:** ~25-40 types defined in the doomed files (mod.rs/task.rs/comment.rs/todos/linearization/redaction/util/api) have LIVE consumers. Step 4 ≠ delete; it = (1) extract every live-consumed type+impl to a surviving home (e.g. relocate TaskId/AIAgentActionId/CodeReview/DiffSetHunk/AgentReviewCommentBatch/ProgrammingLanguage/MessageId/redaction/util-fns/etc.), preserving the crates/ai re-exports (`ai::agent::action::*`/`action_result::*`/AIAgentCitation/FileLocations the blocklist needs), THEN (2) delete the AIConversation-only remainder (AIAgentOutput/conversation.rs/task_store/api convert/etc.). This is multi-session-scale + high-risk; needs a per-type relocation plan (which live consumers, what each type drags in). The investigator's Phase-1 "relocate to ai/util/agent_ids.rs" is the right shape but the survivor set is much larger than its 4-type list. **DO NOT cut blind** — the live-consumer entanglement is exactly what bit Step 3 (cli_controller/handoff). Next session: build the exhaustive per-type extract list (type → defining file → live consumers → transitive deps it drags) before moving anything.

### Step 4 — TRUE SHAPE verified (2026-06-06): mixed-file extraction, the major refactor
Verified defining locations of every live-consumed type. TWO classes:
- **Defined in crates/ai (SURVIVE automatically — just preserve the re-export in a kept module):** AIAgentActionType, AIAgentActionResultType (+ action::*/action_result::* families), AIAgentPtyWriteMode, FileLocations, AIAgentCitation. blocklist/persistence.rs uses these via app mod.rs re-export → keep a thin re-export.
- **Defined in app's DOOMED files, but live-consumed → MUST EXTRACT before deleting:**
  - `task.rs`: **TaskId** (live: cli_controller, terminal/model/block, persistence schema) — but task.rs ALSO defines Task/AIAgentExchange (dead heavy). task.rs is MIXED.
  - `mod.rs` (2703 LoC, MIXED): live = AIAgentActionId, CancellationReason, MessageId, ProgrammingLanguage, AIAgentInput, DiffSetHunk, DiffBase, CurrentHead, AgentReviewCommentBatch, AIAgentContext, ImageContext, UserQueryMode, AIAgentTextSection, SubagentType, ReceivedMessageDisplay, InvokeSkillUserQuery, ServerOutputId; dead = AIAgentOutput/AIAgentOutputMessage/AIAgentOutputStatus/RequestCost/Shared/RenderableAIError/etc.
  - `comment.rs`: **CodeReview** (live: code_review/) — mixed.
  - `todos/mod.rs`: **AIAgentTodoList** (live) — + the dead todo-op logic.
  - `redaction.rs`: **redact_secrets** (live: integration_testing/secret_redaction) — verify if generic.
- **conversation.rs is a leaf** (AIConversation: only consumers = 1 test + 1 doc-comment + the GraphQL `CloudObject::AIConversation` variant which is unrelated). Its helper fns `update_todo_list_from_todo_op`(→ used by task.rs:991 + api convert) + `context_in_exchanges`(→ task.rs, operates on dead AIAgentExchange) tie it to task.rs/api.
- **34 external import sites** use `crate::ai::agent::conversation::{AIConversationId,ConversationStatus,StatusColorStyle}` (re-exported there from conversation_types) — must repoint to `conversation_types::` when conversation.rs goes.

**EXECUTION = green incremental commits (avoids all-or-nothing RED):**
1. Repoint the 34 `agent::conversation::{AIConversationId/ConversationStatus/StatusColorStyle}` imports → `agent::conversation_types::` (per-symbol; AIConversation-type imports are tests→delete). Green.
2. Extract each live type from its mixed file to a surviving home (new `ai/agent/types/` or relocate): TaskId, AIAgentActionId, CancellationReason, MessageId, ProgrammingLanguage, AIAgentInput, DiffSetHunk/DiffBase/CurrentHead, AgentReviewCommentBatch, AIAgentContext, ImageContext, UserQueryMode, AIAgentTextSection(+AgentOutputText/Table/Image deps), SubagentType, ReceivedMessageDisplay, InvokeSkillUserQuery, ServerOutputId, CodeReview, AIAgentTodoList, redact_secrets. Each extract = its own green commit (move type+impls+transitive deps, fix referencing sites). **CAUTION: re-verify each "live consumer" is truly live — code_review/AgentReviewCommentBatch/DiffSet* may themselves be AI-dead and removable, shrinking the extract set. Check before extracting.**
3. Once mixed files hold only dead heavy types: delete conversation.rs + the AIAgentOutput model in mod.rs + task.rs heavy (Task/AIAgentExchange) + task_store + api/convert + comment dead + todos dead + linearization + util + telemetry + conversation_yaml + RequestInput/ResponseStreamId + persistence agent cluster (agent.rs/AgentConversation/multi_agent_conversations/ModelEvent UpdateMultiAgentConversation·DeleteAIConversation·DeleteMultiAgentConversations + sqlite handlers) + AIAgentExchangeId. Big shrink + clears ~120 dead-core warnings.
This is THE major refactor — its own focused session(s); do step 2 one-type-at-a-time, green each.

### Step 4 increment 1 DONE (2026-06-06, `896afb77`): repointed 33 external conversation:: imports → conversation_types:: (zero binary delta). conversation.rs re-export role decoupled. NEXT increment 2 = extract live types from mixed files one-per-green-commit (start with the simplest leaf type; re-verify each consumer is truly live first).

### Step 4 increment 3 ATTEMPTED + REVERTED (2026-06-06) — conversation.rs is NOT a deletable leaf
Tried deleting conversation.rs alone (after relocating its 2 free helper fns). Cascade (6+ errors) proved it's mutually-referential, not a leaf:
- conversation.rs holds `impl AIAgentExchange { init_output, ... }` (and likely impl Task) methods that **surviving task.rs** calls (task.rs:316/447 `init_output`). AIAgentExchange's def is in mod.rs but its methods are split into conversation.rs.
- `telemetry.rs:3` imports from `super::conversation`.
- (todos relocation needed the `api::message` path fixed — minor.)
**Conclusion:** conversation.rs + mod.rs-heavy (AIAgentOutput/AIAgentExchange/Task impls) + task.rs + task_store + api/ + comment + linearization + telemetry + util form ONE mutually-referential mass. You CANNOT delete any one file alone — task.rs uses AIAgentExchange::init_output (conversation.rs), conversation.rs uses Task/task_store/api, etc. **Deletion is all-or-nothing for the mass.** Therefore the ONLY safe path:
1. Create a NEW standalone module (e.g. `ai/agent_ids.rs`) with the live-consumed leaf types that have NO deps on the dead mass: TaskId, AIAgentActionId, CancellationReason, MessageId, ServerOutputId, ProgrammingLanguage, UserQueryMode, AIAgentContext, ImageContext, SubagentType, ReceivedMessageDisplay, InvokeSkillUserQuery, AIAgentTextSection(+AgentOutputText/Table/Image/MermaidDiagram deps), DiffSetHunk/DiffBase/CurrentHead, AgentReviewCommentBatch, CodeReview, AIAgentTodoList, AIAgentTodo/AIAgentTodoId, redact_secrets, AIAgentInput(?). Each must be copied with its impls + the transitive leaf deps it needs, and must NOT pull in AIConversation/AIAgentOutput/Task/AIAgentExchange. SOME of these (AIAgentInput, AIAgentTextSection) may transitively need dead types → verify; if so they're harder.
2. Repoint ALL external consumers to the new module.
3. THEN delete the entire core mass (conversation.rs + mod.rs heavy + task.rs + task_store + api/ + comment + linearization + telemetry + util + todos-dead + RequestInput/ResponseStreamId + persistence agent cluster + AIAgentExchangeId) in ONE commit.
This is genuinely multi-session. Reverted to clean green `62b82f7c`. The increments that DID land green: import repoint (`896afb77`) + test-file deletion (`62b82f7c`). Next session: build the standalone-leaf-types module incrementally (each type that extracts cleanly = green commit), discovering which types are truly leaf vs transitively-dead-coupled.

### Step 4 increment 4 DONE (2026-06-06, `ccb40bde`): leaf-type extraction — the standalone module is BUILT
Investigator-mapped every doomed-module symbol with external `crate::ai::agent::` consumers, then verified each consumer is genuinely surviving (not itself AI-dead). Result: **extracted 9 types + 1 fn + 6 icon fns** into two NEW sibling modules OUTSIDE the doomed dir:
- **`app/src/ai/agent_types.rs`** — TaskId, AIAgentActionId, CancellationReason, ProgrammingLanguage, ImageContext, CurrentHead, DiffBase, DiffSetHunk, AgentReviewCommentBatch, `redact_secrets`. All verified clean leaves (no transitive dep on AIConversation/AIAgentOutput/Task/AIAgentExchange). `AgentReviewCommentBatch` uses surviving `code_review::comments::ReviewComment` (alias CodeReviewComment) + DiffSetHunk (co-moved). The From impls into `warp_multi_agent_api`/`diff_hunk_api` proto crates survive.
- **`app/src/ai/agent_icons.rs`** — the 6 status icon fns (icons.rs relocated verbatim).
- Doomed `mod.rs`/`task.rs` re-export the moved types (`pub use crate::ai::agent_types::…`) so internal doomed code is unaffected; `icons.rs`/`redaction.rs` are now re-export shims. Repointed ~22 external consumer files. TaskId tuple-constructions inside task.rs → `TaskId::new(...)` (private field now in another module).
- **VERIFIED NOT needing extraction (consumer is AI-dead or test-only → dies WITH the mass):** `AIAgentInput` (only consumer = blocklist/request_input.rs, itself AI-dead, NOT in the blocklist keep-list); `AIAgentTodoList`+`AIAgentTodo`+`AIAgentTodoId` (only a dead `lib.rs:125` re-export, zero downstream); `MessageId` (only a test + drags AIAgentOutputMessage); `comment::CodeReview` (NO external consumer — investigator's first-pass "code_review_view.rs" was wrong). `ServerOutputId`/`InvokeSkillUserQuery`/`AIAgentContext`/`UserQueryMode`/`SubagentType`/`ReceivedMessageDisplay`/`AIAgentTextSection`+output-text/table/image deps = NO external consumers, die with mass.
- 3-gate **0/0/0**, no net new warnings, zero binary delta (pure relocation — fold the size row into the eventual mass-deletion commit).

### Step 4 increment 4b DONE (2026-06-06, `24b5eeff`): doomed module fully ISOLATED — precondition for the cut met
Severed the LAST live consumers of `ai/agent/`. Now the only external `crate::ai::agent::` refs are the 2 DEAD consumers that delete WITH the mass. Did:
- Relocated `conversation_types.rs` OUT of the doomed dir → `app/src/ai/conversation_types.rs` (it is KEPT). Repointed all 34 `crate::ai::agent::conversation_types::` + 5 `api::ServerConversationToken` consumers → `crate::ai::conversation_types::`. Its lone internal dep (agent::icons) → agent_icons.
- Removed the dead `AIAgentAttachment::DiffHunk` vestige in code_review_view.rs (built into a discarded `_attachment` — live behaviour is the `<change:…>` ref insertion + agent-mode lock). → AIAgentAttachment loses its last live consumer, dies with mass.
- Repointed the 3 files reaching crate-level agent types through the doomed app re-export → real crate path `ai::agent::action::`/`ai::agent::FileLocations` (persistence.rs, terminal/view.rs, pty_controller.rs).
- Deleted now-unused `icons.rs`+`redaction.rs` shims + mod decls; trimmed dead FileLocations re-export from agent/mod.rs.
- 3-gate 0/0/0, 214/200/215 warnings (no net new), 0 binary delta.

### Step 4 increment 5 — THE MASS CUT ✅ DONE (`ed89397e`, 2026-06-06)
Deleted `app/src/ai/agent/` (−11,495 LoC) + dead consumers (request_input.rs, orphan blocklist_filter_tests.rs, now-dead response_stream_id.rs) + lib.rs re-exports + `mod agent`. 3-gate **0/0/0**. Binary 704.3 → 703.6 MB (−0.70 MB — mass was already mostly linker-dead from sessions 55–57 consumer-severing). Warnings 214/200/215 → **108/88/109** (~106 dead-core warnings gone). conversation_types/agent_types/agent_icons survive as extracted siblings. **SURPRISE:** the persistence `AgentConversation` cluster did NOT break — it uses its OWN diesel model types (decoupled from the deleted module), so it still compiles. It is now DEAD (sole emitter was the deleted conversation.rs:2810 `UpdateMultiAgentConversation`).

### Step 4 increment 5b ✅ DONE (`302a2ba1`, 2026-06-06) — STEP 4 COMPLETE
Deleted the dead persistence AgentConversation cluster: app/src/persistence/agent.rs + `mod agent`; mod.rs `multi_agent_conversations` field + 3 ModelEvent variants + AgentConversation/api imports; sqlite.rs 3 handler arms + read/load; block_list.rs dead `delete_ai_conversation`. KEPT the `persistence` crate model types + agent_conversations/agent_tasks schema tables (DB compat). 3-gate **0/0/0**. Warnings 108/88/109 → **107/87/108**. **Binary 703.6 → 697.7 MB = −5.85 MB** — the REAL Step-4 win: unlike the ai/agent/ dir (linker-dead), this cluster was LIVE-LINKED via the writer-thread `persist()` match (diesel insert/select machinery + AgentConversationData proto). **STEP 4 (core AI agent strip) COMPLETE** across increments 1/2/4/4b/5/5b: AIConversation keystone + entire ai/agent/ module (~12K LoC) + persistence cluster gone; conversation_types/agent_types/agent_icons survive as extracted siblings. Total Step-4 binary: 704.3 → 697.7 MB (−6.55 MB), warnings 214→107.

**(historical) NEXT = increment 5b: delete the now-dead persistence AgentConversation cluster** (per no-dead-code policy). Surface (all contained): `persistence/agent.rs` (whole file, read/write_agent_conversations + NewAgentConversation); `persistence/mod.rs` (`use self::model::{AgentConversation, AgentConversationData}` :40, `multi_agent_conversations: Vec<AgentConversation>` field :181, ModelEvent::{DeleteAIConversation :295, UpdateMultiAgentConversation :298, DeleteMultiAgentConversations :303} — the 3 variants matched ONLY in sqlite.rs); `persistence/sqlite.rs` (import :90, 3 handlers :649/:653/:664, read call :3091, load field :3109); the AgentConversation/AgentConversationData/AgentConversationRecord/AgentTaskRecord model types. **KEEP the sqlite agent_conversations/agent_tasks TABLE defs (DB compat, precedent: ai_queries `ea27b66b`).** CAUTION: the `model` module isn't a model.rs/model/ dir — likely a `mod model` block or macro-generated inside persistence; locate AgentConversation's real def before deleting. Verify no exhaustive `match ModelEvent` elsewhere breaks when the 3 variants go (grep showed only sqlite.rs matches them). Diesel-delicate — do it as its own green commit.

### Step 4 increment 5 — THE MASS CUT (mapped 2026-06-06, ready to execute as ONE green commit)
Full cut surface verified — `AIConversation`/`AIAgentOutput`/`Task` mass has ZERO live CODE consumers anywhere (only doc-comment mentions + the unrelated `warp_graphql::CloudObject::AIConversation` graphql variant, which survives). Delete in one commit:
1. **`rm -rf app/src/ai/agent/`** (conversation.rs/mod.rs/task*/task_store/util/linearization/comment/todos/api/telemetry/conversation_yaml — ~12K LoC). Remove `pub(crate) mod agent;` from `app/src/ai/mod.rs:4`.
2. **lib.rs**: drop `:125 pub use ai::agent::todos::AIAgentTodoList;` (zero downstream). KEEP `:126` (crate-level AIAgentActionResultType/FileEdit/TodoOperation).
3. **blocklist DEAD consumer**: delete `ai/blocklist/request_input.rs` + `ai/blocklist/mod.rs:6` (`mod request_input;`) + `:8` (`pub use request_input::RequestInput;`). RequestInput blast radius = just that re-export (no live user).
4. **test DEAD consumer**: delete `terminal/view/blocklist_filter_tests.rs` (imports AIConversation/MessageId) + its mod decl in terminal/view.rs (or view_tests).
5. **persistence agent cluster** (write-only/dead since AIConversation can't be built): `persistence/agent.rs` (whole file); `persistence/mod.rs` (`pub mod agent;` :5, the AgentConversation/AgentConversationData import :40, `multi_agent_conversations` field :181, ModelEvent::{DeleteAIConversation :295, UpdateMultiAgentConversation :298, DeleteMultiAgentConversations :303}); `persistence/sqlite.rs` (3 ModelEvent handlers :649/:653/:664, `read_agent_conversations` call :3091 + the load-struct field :3109); the AgentConversation/AgentConversationData/AgentConversationRecord/AgentTaskRecord model types. **KEEP the sqlite agent_conversations/agent_tasks TABLE defs for DB compat (precedent: ai_queries table kept in `ea27b66b`).** Verify the ModelEvent enum + emitters (anything still EMITTING these 3 variants? grep — should be 0 since AIConversation construction is gone).
6. Fix residual break sites (touched_repos.rs doc-comment optional; any `crate::AIAgentTodoList` lib re-export consumers — none).
7. 3-gate green. **Expect a REAL binary shrink** (convert_conversation/convert_from MAA-proto machinery + task_store may be partially live-linked via the agent module surface) — measure + add the build-size row HERE (folds in the increments-1/2/4/4b 0-delta relocations).
**Risk:** the persistence ModelEvent enum edit + sqlite load-chain is the delicate part (diesel). Do the dir-delete + mod-decl + dead-consumer deletes first (compiler enumerates persistence break sites), then surgically remove the persistence cluster. All-or-nothing — cannot land half.

**(superseded) NEXT = the mass deletion (increment 5).** With every SURVIVING consumer now pointed at `agent_types`/`agent_icons`, the doomed `app/src/ai/agent/` dir + its AI-dead cascade consumers can be deleted as ONE green commit. The cascade to delete WITH it: `ai/blocklist/request_input.rs` (+ blocklist/mod.rs re-export of RequestInput) — and re-verify whether `cli_controller.rs` (uses RequestInput? it's LIVE via terminal core — check: cli_controller imports TaskId+AIAgentActionId now repointed, but does it import RequestInput/other doomed types?), `terminal/view/blocklist_filter_tests.rs` (MessageId test), `lib.rs:125` AIAgentTodoList re-export, and the persistence agent cluster (agent.rs/AgentConversation/multi_agent_conversations field + ModelEvent UpdateMultiAgentConversation·DeleteAIConversation·DeleteMultiAgentConversations + sqlite handlers). Before cutting: run a fresh investigator pass to enumerate EVERY remaining external `crate::ai::agent::` import (non-extracted symbols) + classify each consumer file LIVE-vs-DEAD; the LIVE ones are the blocker (must be 0 before the dir delete), the DEAD ones delete in the same commit. Likely-remaining non-extracted external imports to resolve: AIAgentPtyWriteMode, AIAgentInput, AIAgentContext, comment::CodeReview, MessageId, AIAgentTodo (recheck each — some are crate-level `ai::agent::` re-exports, NOT app doomed).

---

## cloud_preferences strip (settings→cloud sync) — session 58

**Increment A DONE (`7fb92e80`):** removed the settings-sync TOGGLE (`IsSettingsSyncEnabled` / `CloudPreferencesSettings` group) + all UI/auth consumers — SettingsSyncWidget (account "Settings sync" row), MainPageAction::ToggleSettingsSync + binding machinery, LocalOnlyIconState::for_setting → always Hidden, keybindings/features_page/workspace branches collapsed, auth clear-on-logout block, SETTINGS_SYNC_FLAG, registrations. 3-gate 0/0/0, warnings net-neutral. The `CloudPreference` cloud-object model is now DEAD (sync never triggers) but still compiled.

**Increment B TODO — remove the now-dead CloudPreference cloud-object MODEL (cross-crate):**
- `app/src/settings/cloud_preferences.rs` — DELETE the whole file (CloudPreference/CloudPreferenceModel/Preference/Platform) + `pub mod cloud_preferences;` + `pub use cloud_preferences::*;` in settings/mod.rs.
- `cloud_object/mod.rs` — `ServerCloudObject::Preference(ServerPreference)` variant (:929) + match arms (:945/:967/:999) + the CloudPreferenceModel import (:37).
- `cloud_object/model/persistence.rs` — `ServerCloudObject::Preference` arm (:505) + `get_all_cloud_preferences_by_storage_key` (:1280) + import (:28).
- `persistence/sqlite.rs` — `JsonObjectType::Preference` deserialize arm (:2830) + import (:93).
- `settings/privacy.rs:552` — `get_all_cloud_preferences_by_storage_key` caller (READ what it does first — privacy.rs may apply synced privacy prefs; confirm removable/dead).
- `cloud_object/model/model_tests.rs` — CloudPreference::new test.
- **CROSS-CRATE (run `cargo check -p warp_server_client` + check graphql):** `crates/warp_server_client/src/cloud_object/mod.rs:180` `JsonObjectType::Preference` variant; `crates/graphql/src/api/generic_string_object.rs:18` `JsonPreference`. Removing the JsonObjectType variant ripples to any From/match in those crates.
- Likely a real binary shrink (the Preference cloud-object deserialize path is live-linked like the AgentConversation cluster was). Gate: 3-gate + `-p warp_server_client`. Note `ServerPreference` type def location (probably warp_server_client) — keep or remove with the variant.

**Inc B ENTANGLEMENT found (session 58):** `get_all_cloud_preferences_by_storage_key` has one live consumer — `settings/privacy.rs:552` `handle_warp_drive_objects_loaded`, which reads synced cloud prefs (IsTelemetryEnabled/IsCrashReportingEnabled/IsCloudConversationStorageEnabled storage keys) to reconcile telemetry/crash/cloud-conversation-storage between cloud and local PrivacySettings, then (else-branch) syncs local → WarpDrivePrivacySettings + cloud via `maybe_sync_local_prefs_to_cloud`. With cloud-sync gone, cloud_prefs is always empty → always the else-branch → that branch's cloud push is also dead. This is the **Warp Drive privacy-sync** subsystem (plan §3 "Cloud sync / Warp Drive" + auth privacy). **RECOMMENDATION: do Inc B together with the Warp Drive privacy-sync strip** — neuter `handle_warp_drive_objects_loaded` (drop cloud-pref reconciliation + maybe_sync_local_prefs_to_cloud; keep local PrivacySettings init), THEN remove get_all_cloud_preferences_by_storage_key → CloudPreference model → ServerCloudObject::Preference + gql_convert arms (gql_convert.rs:1055/1109 ServerPreference::try_from_gql) + cross-crate JsonObjectType::Preference (warp_server_client) + JsonPreference (graphql). ServerPreference = cloud_object/mod.rs:1030 type alias (dies with the variant). Don't half-cut privacy.rs alone.

---

## Teams strip (session 61) — IN PROGRESS

Teams = cloud-backend org overlay (server-populated via authenticated `workspaces_metadata` fetch). No backend → **no team can ever exist at runtime** (`has_teams()` always false, `current_team()` always None). Local **Workspace** ≠ Team — KEEP Workspace.

- **inc 1 DONE (`88ca0334`, −1.05 MB):** dead `WorkspaceClient` billing client — deleted server_api/workspace.rs (trait+impl: stripe portal / addon credits / usage-based pricing / ai-overages refresh + 5 graphql op-builders) + ServerApiProvider::get_workspace_client + UserWorkspaces workspace_client field + 5 billing wrappers + 7 event variants + 3 ctor params; updated ~30 test files. KEEP AiOverages/are_overages_remaining (fed by initial fetch).
- **inc 2a DONE (`ff778833`, ~0 linker-invisible):** 22 dead team-CRUD `UserWorkspacesEvent` variants (invite/domain/discoverability/ownership/role/upgrade-link) — zero emitters (removed s58) + zero subscribers.
- **inc 2b DONE (`cbc4b5c4`, ~0):** team-discovery scaffolding (joinable_teams field + update_joinable_teams + num/total accessors + FetchDiscoverableTeams events + WorkspacesMetadataResponse.joinable_teams + gql_convert build + From<GqlDiscoverableTeamData> + DiscoverableTeam struct). KEEP cynic GqlDiscoverableTeamData.
- **DECISION (session 61, user):** Sublight removes teams **entirely, even for API-key-authed users** (it's not Warp). Auth analysis: the workspace/team fetch is gated on `is_logged_in()` (credentials present); the live cred path is **API-key auth** (`auth_state.rs:108`), so teams CAN populate when authed — therefore teams is a real feature removal (via gql-reshape), not dead-code. KEEP the workspace fetch (serves the non-team authed workspace list + experiments).
- **inc 2c STARTED (`<cloud_object commit>`):** cloud_object `Space::name` no longer resolves a team name (`Space::Team{..}` → `"Team"`), dropping one `team_from_uid` caller. NOTE: `team_from_uid`/`_across_all_workspaces` still have INTERNAL callers (tier-limit/capacity methods user_workspaces.rs:198/235/559) → not yet deletable.
- **REMAINING keystone — ORDERED execution plan (each a green increment):**
  1. **Tier-limit/capacity cluster** — `is_at_tier_limit_for_object_type`/`_some_warp_drive_objects`/`has_capacity_for_shared_notebooks`/`_workflows` (read `team.billing_metadata.tier` via team_from_uid) → collapse to `true`/unlimited at their drive-create call sites (server/cloud_objects/update_manager create_notebook/create_workflow). Frees team_from_uid's internal callers.
  2. **current_team consumers (6):** ai/mcp/templatable_manager:1292, drive/workflows/ai_assist:135, search/command_search/view:519, mcp_servers/list_page:1232, workflows/workflow_view:2588, workspace/view:14764 — each `if let Some(team)=current_team()` → None path. **current_team_uid (2):** workflows/categories:570, workspace/view:14304 → None. **has_teams (1):** mcp is_shareable:318 → false (cascades: MCP team-sharing UI + get_first_team_space_id dead → remove). **team_spaces (1):** notebook:1314 → empty (remove move-to-team-space menu branch).
  3. Remove the now-0-caller read accessors: current_team/current_team_mut/current_team_uid/has_teams/team_spaces/team_from_uid/_across_all_workspaces.
  4. **Policy accessors:** collapse the live-caller ones (ai_autonomy_settings/is_enterprise_secret_redaction*/is_ai_allowed_in_remote_sessions/is_telemetry_force_enabled/get_ugc*/is_next_command_enabled/is_git_operations_ai_enabled/get_cloud_conversation_storage*) at call sites to their no-team defaults; delete the 0-caller ones.
  5. **Space::Team variant** (cloud_object) — once no constructor remains, remove the variant + its match arms (Space → {Personal, Shared}).
  6. **The struct keystone:** `Workspace.teams: Vec<Team>` field → drop + reshape gql_convert (stop building teams from the response; the `into()` for Workspace drops teams) + persistence; then delete `Team`/`TeamMember` (KEEP `MembershipRole` — used by WorkspaceMember.role; KEEP cynic graphql types) + `team_tester.rs` (TeamTesterStatus + its TeamUpdateManager subscription). gql-reshape is the delicate part — verify `crates/graphql` + run `-p warp` + persistence.
  - **(former map; superseded by the ordered plan above):**
  - **HIGH-RISK (caller-collapse in kept features):** `has_teams`/`current_team`/`team_spaces`/`team_from_uid` woven into MCP server-scoping (mcp_servers/list_page.rs:318/1232), notebook move-to-space (notebook.rs:1314), cloud_object Space.name (cloud_object/mod.rs:1137), drive/workflows (ai_assist/categories/workflow_view), command_search. Collapse each team-branch to the no-team path.
  - **Live-caller policy accessors** (read `team.organization_settings`, called by live features but always return defaults): `ai_autonomy_settings`(7), `is_enterprise_secret_redaction_enabled`(5)+`get_..._regex_list`(5), `is_ai_allowed_in_remote_sessions`(3)+regex(1), `is_telemetry_force_enabled`(2), `get_ugc_collection_enablement_setting`(2), `is_next_command_enabled`(1), `is_git_operations_ai_enabled`(1), `get_cloud_conversation_storage_enablement_setting`(1) — must collapse at the call sites (the caller takes the default branch), not just delete.
  - **0-caller policy accessors (verify no internal caller, then delete):** `ai_allowed_for_current_team`, `is_prompt_suggestions_toggleable`, `is_code_suggestions_toggleable`, `sandboxed_agent_settings`, `is_ai_autonomy_allowed`, `is_anyone_with_link_sharing_enabled`, `is_direct_link_sharing_enabled`.
  - **`Team` struct + `Workspace.teams: Vec<Team>` + DiscoverableTeam/TeamMember/MembershipRole + team_tester.rs + joinable_teams field + FetchDiscoverableTeams event:** the keystone. **TRAP: `warp_graphql` Team types are cynic schema-bound + the `workspaces_metadata` query RESPONSE carries the teams array** → `Workspace.teams` + app `Team` are load-bearing for gql DESERIALIZATION (even when always empty). Removing them = reshaping gql_convert (workspaces/gql_convert.rs:35/42/91-112) + persistence (sqlite.rs:2937 MembershipRole) + ~50 refs. Verify graphql crate before deleting app types.
  - **KEEP (verified, misnamed):** `TeamUpdateManager` (= workspace-metadata POLLING) + `TeamClient` trait + `workspaces_metadata()` (= the core workspace fetcher) + `WorkspaceMember.role` (generic workspace membership) + `TeamsChanged` event + PrivacySettings coupling. Consider renaming TeamUpdateManager→WorkspacesMetadataPollingManager, TeamClient→WorkspacesMetadataClient for clarity (optional).

## server/ scope (session 58) — backend spine, NO clean incremental cut yet

32 files / 6.6k LoC / 181 fan-in (plan's old "55/40k/454" is stale — bulk stripped). Core = `server_api.rs` (53.7K: ServerApi HTTP client + send_graphql_request transport + ServerApiProvider singleton exposing 7 client traits). **Every client trait is held by a still-live model**, so none is a clean cut:
- **TeamClient** (team.rs, 727 LoC) → `UserWorkspaces` (core model, ~15 team-method wrappers at user_workspaces.rs:833-1047) + `TeamUpdateManager` (lib.rs:1427 + auth/mod.rs:180 + settings_view/main_page.rs:801 + test mocks). 2 get_team_client callers (lib.rs:1139 UserWorkspaces::new, 1428 TeamUpdateManager::new).
- **ReferralsClient** (referral.rs) → referral_theme_status.rs (KEPT theme-unlock).
- **ManagedSecretsClient** (managed_secrets.rs 319) → ManagedSecretManager (lib.rs:1176) → used by MCP (KEPT) + aws_credentials.
- WorkspaceClient/AuthClient/BlockClient/AIClient(14 callers) → core models.

**(SUPERSEDED by the "Teams strip (session 61)" section above — team CRUD removed s58, billing client + event variants removed s61 inc1/2a; TeamClient is now 109 LoC = the kept workspaces_metadata fetcher. Remaining = the Team-struct/accessor collapse mapped above.)** ~~Teams backend strip (most-contained, ~MEDIUM, own focused session): (1) remove team-management methods from UserWorkspaces (send_team_invite_email/reset_invite_links/invite-domain-restrictions/rename/discoverability/ownership/roles — ~15 wrappers + team_client field + param in both new() ctors) + fix the settings_view/main_page.rs:801 + auth/mod.rs:180 TeamUpdateManager refs; (2) delete TeamUpdateManager (workspaces/update_manager.rs) + its lib.rs/auth/test regs; (3) delete TeamClient trait + team.rs + ServerApiProvider::get_team_client + the 2 lib.rs caller args; (4) cascade DiscoverableTeam/WorkspacesMetadataWithPricing/invite types if orphaned. CAUTION: UserWorkspaces is load-bearing (local workspace state) — keep workspaces_metadata if it backs local workspace listing; verify each team method is truly UI-stripped-dead before removing. Risky (core model) — do with fresh focus, not tail-of-session.~~

**Peripheral server/ targets (cleaner, smaller):** voice_transcriber + ai/voice (editor voice→server transcription, a removable feature, MEDIUM); network_logging/network_log_* (generic HTTP inspector — likely KEEP). telemetry/events.rs metadata structs are threaded through ~10 files (NOT dead — defer). retry_strategies plan-deferred.

---

## VOICE feature — EXACT full map (session 58, verified file-by-file)

Voice = local mic capture (cpal) → WAV → **warp-server `/ai/transcribe`** (Wispr/OpenAI) → text inserted into editor/terminal input. **Server-backed (needs Warp backend) → strip target.** Gated by Cargo feature `voice_input` (optional dep; `gui = ["voice_input"]`; NOT in default features, so default + `--tests` gates already compile with it OFF).

### Strategy: 2 phases. Phase 1 (drop feature) auto-removes all cfg-gated wiring; Phase 2 deletes the unconditional residue.

**A. Cargo feature `voice_input` (drop in Phase 1):**
- `app/Cargo.toml:206` `voice_input = { workspace = true, optional = true }` · `:614` `gui = [… "voice_input"]` · `:716` `voice_input = ["dep:voice_input"]`
- `Cargo.toml:67` workspace member `voice_input = { path = "crates/voice_input" }`
- **`crates/voice_input/` (422 LoC)** — the whole audio-capture crate (cpal mic, rubato resample, WAV/base64): VoiceInput singleton, VoiceInputState, VoiceInputToggledFrom, VoiceSessionResult, StartListeningError. DELETE crate.

**B. `#[cfg(feature="voice_input")]`-GATED (vanish automatically when feature dropped — confirmed gated):**
- `editor/view/mod.rs:6-7` `mod voice;` → the **whole 589-line `editor/view/voice.rs`** + mod.rs voice items at :60/:104/:123/:1036 (EditorAction::ToggleVoiceInput, fields, VOICE consts region).
- `lib.rs:1387` `VoiceInput::new` singleton reg.
- `root_view.rs:195-200` global actions (abort_voice_input / maybe_stop_active_voice_input) + `:1169` fns + key handling (1662-1685).
- `terminal/universal_developer_input.rs` ToggleVoiceInput action+event+render (204/211/238/535/568/604) + set_voice_is_listening.
- `terminal/view/action.rs:247` ToggleCLIAgentVoiceInput.
- `terminal/input.rs:3274` handler.
- key sites (verify gated): `editor/view/element.rs:458`, `terminal/alt_screen/alt_screen_element.rs:556`, `terminal/block_list_element.rs:2699` (VoiceInputToggledFrom::Key).
- ~15 `*_tests.rs` `#[cfg(feature="voice_input")] add_singleton_model(voice_input::VoiceInput::new)` regs.

**C. UNCONDITIONAL (compiles with feature off → must DELETE explicitly in Phase 2):**
- `app/src/voice/` (mod.rs + transcriber.rs, 49 LoC) — `Transcriber` trait + `VoiceTranscriber` singleton. lib.rs:86 `mod voice;`.
- `app/src/ai/voice/` (mod + transcribe/ api types, ~160 LoC) — TranscribeRequest/Response/Provider/OpenAIProperties/WisprProperties. ai/mod.rs:21 `pub(crate) mod voice;`.
- `app/src/server/voice_transcriber.rs` (41 LoC) — ServerVoiceTranscriber (impl Transcriber via server_api.transcribe). server/mod.rs:12.
- `lib.rs:1389` `VoiceTranscriber::new(ServerVoiceTranscriber::new(...))` reg (NOT gated — only the VoiceInput reg above it is) + imports :139/:148.
- `server/server_api.rs` — `transcribe()` method (931-975) + `TranscribeError` enum (334) + TranscribeRequest/Response imports (44).
- `settings/ai.rs` — `VoiceInputToggleKey` enum (105-213+) + `is_voice_input_enabled()` (1532) + settings `dismissed_voice_input_new_feature_popup` (865) / `explicitly_interacted_with_voice` (878) + interacted logic (1782-1805).
- `workspaces/workspace.rs:319` `is_voice_enabled` field (WarpAiPolicy) + `gql_convert.rs:170` + `user_workspaces.rs:450-462` `is_voice_enabled()` (uses `cfg!(feature="voice_input")`).
- `editor/view/mod.rs:133` `VOICE_ERROR_TOAST_TEXT` (+ VOICE_LIMIT_HIT) consts + `:1355` `pub use …VoiceTranscriber` re-export — verify gated vs not.

**Scope: 58 files touch "voice"; ~30 are cfg-gated (vanish via feature drop) or test-regs; ~12-15 unconditional files need explicit edits + 1 crate deletion.** Cascade to watch: FeaturePopup "Try Voice Input" (NewFeaturePopupLabel) usage; AISettings entered_agent_mode_num_times is SHARED (keep); UserWorkspaces is load-bearing (only remove is_voice_enabled method + policy field). 3-gate + the gui build (`--features gui` pulls voice_input today — after the strip, gui must drop it). This is a real MEDIUM (whole crate + Cargo feature + 2 input surfaces + settings + workspace policy), NOT a small clean feature.

### VOICE feature — ✅ DONE (session 58, `e92903ea` + `c36a60b4`)
Executed the full map above. Phase 1: dropped Cargo feature `voice_input` + `gui=["voice_input"]` + deleted `crates/voice_input/`. Phase 2: deleted module trees (app/src/voice/, ai/voice/, server/voice_transcriber.rs, editor/view/voice.rs) + server_api transcribe()/TranscribeError + swept all 82 `#[cfg(feature="voice_input")]` sites (deleted gated items, unconditionalized no-op fallbacks, collapsed cfg_if to disabled branch). Removed dead VoiceTranscriptionOptions/VOICE consts/request-usage voice methods. **3-gate 0/0/0, warnings 106/87/106 = baseline, −2.05 MB → 695.4 MB.** KEPT inert (no warning, removing = signature surgery): VoiceStateUpdated editor event + input handler; voice_input_toggle_key_code method + VoiceInputToggleKey setting + toggle-key element fields (allow(dead_code)) threaded through with_input_editor_icons. **Residual cleanup (optional follow-up):** unwind voice_input_toggle_key_code from with_input_editor_icons signature (editor/element/block_list/alt_screen) + drop VoiceInputToggleKey setting + VoiceStateUpdated event + mic_button/set_voice_is_listening UDI residue.

---

## TEAMS backend strip — EXACT map (session 58, CORRECTS the earlier over-scoped plan)

**3 wrong-picture traps the earlier plan would have hit — DO NOT remove these:**
1. `TeamClient::workspaces_metadata()` = the CORE workspace-fetch (update_manager.rs:150/207 load workspaces through it; UserWorkspaces wraps the result at 1251/1293). **KEEP the trait + this method + get_team_client (TeamUpdateManager uses it).**
2. `TeamUpdateManager` (workspaces/update_manager.rs) is MISNAMED — it's the **workspace-metadata POLLING** manager (start/stop_polling_for_workspace_metadata_updates, refresh_workspace_metadata, poll_for_workspace_metadata_changes, on_workspaces_updated, set_current_workspace_uid). Consumers auth/mod.rs:180 (stop-poll on logout) + settings main_page:801 (refresh) are workspace ops. **KEEP entirely.**
3. UserWorkspaces team READ-accessors are LIVE: has_teams (5 callers), current_team (7), current_team_uid (2), team_spaces (1), team_from_uid (1), update_joinable_teams (1 via update_manager:291). **KEEP.**

**REMOVABLE = dead team-CRUD (all 0 live callers — teams UI stripped at `ac8a95c3`):**
- team.rs `TeamClient` trait + `impl for ServerApi`: remove the 15 CRUD methods (create_team, leave_team, rename_team, transfer_team_ownership, set_team_member_role, delete_team_invite, get_discoverable_teams, remove_user_from_team, add/delete_invite_link_domain_restriction, send_team_invite_email, set_is_invite_link_enabled, reset_invite_links, set_team_discoverability, join_team_with_team_discovery) — KEEP workspaces_metadata.
- UserWorkspaces CRUD wrappers + on_* callbacks (user_workspaces.rs ~816-1166): team_created, remove_user_from_team, add/delete_invite_link_domain_restriction(+on_), send_email_invites(+on_email_invite_sent), set_is_invite_link_enabled(+on_), reset_invite_links(+on_), set_team_discoverability(+on_), join_team_with_team_discovery(+on_), fetch_discoverable_teams(+on_), transfer_team_ownership(+on_), set_team_member_role(+on_), delete_team_invite(+on_), generate_upgrade_link(+on_, billing), generate_stripe_billing_portal_link(+on_, workspace_client billing). Then UserWorkspaces.team_client FIELD becomes dead → drop field + new() param (both ctors) + lib.rs:1139 get_team_client arg (lib.rs:1420 TeamUpdateManager arg STAYS).
- **CASCADE (the risk): UserWorkspacesEvent variants** for the removed flows (AddDomainRestrictions{Rejected,Success}, DeleteDomainRestriction*, EmailInvite{Rejected,Sent}, ToggleInviteLinks*, ResetInviteLinks*, ToggleTeamDiscoverability*, JoinTeamWithTeamDiscovery*, FetchDiscoverableTeams*, TransferTeamOwnership*, SetTeamMemberRole*, DeleteTeamInvite*, GenerateUpgradeLink*, GenerateStripeBillingPortalLink*) — verify their HANDLERS (workspace/view.rs etc.) are already dead/removable before deleting the variants.
- Cascade types (remove if orphaned): CreateTeamResponse, DiscoverableTeam, MembershipRole(?), invite-link-domain-restriction types. KEEP WorkspacesMetadataWithPricing/WorkspacesMetadataResponse/Team/Space.
- Tests: user_workspaces_tests.rs team-CRUD tests + mockall expect_* for removed trait methods.

**Scope: ~700 LoC, core model (UserWorkspaces) + UserWorkspacesEvent handler cascade. Execution order:** (1) UserWorkspaces CRUD wrappers+callbacks+field+events (verify event handlers dead first) → (2) team.rs trait/impl CRUD methods + cascade types → (3) tests. Green per increment. Do with fresh focus — UserWorkspaces is load-bearing; the event-variant→handler cascade is the trap.

**TEAMS map — 2 more KEEP-nuances (verified):**
4. `generate_stripe_billing_portal_link` + `on_generate_stripe_billing_portal_link` are LIVE (main_page.rs:92 Account page, uses workspace_client) — **KEEP**. (generate_upgrade_link + on_generate_upgrade_link = 0 callers, dead, remove.)
5. `on_workspaces_updated` (us.rs:775) + `on_update_workspace_metadata` (1244) are SHARED callbacks — used by the dead CRUD methods AND live workspace-update paths (1297, 1337). **KEEP both.** So removal is per-method (drop each dead CRUD fn + its DEDICATED on_* callback like on_email_invite_sent/on_team_discoverability_set), NOT a contiguous block-delete — the shared callbacks are interleaved.

**Verdict:** Teams strip is precisely mapped (5 traps caught: workspaces_metadata, TeamUpdateManager, read-accessors, stripe-billing, shared-callbacks all KEEP). Removable = ~15 dead TeamClient CRUD methods + ~18 dead UserWorkspaces CRUD fns/dedicated-callbacks + team_client field + event variants + cascade types + tests (~700 LoC). Intricate core-model surgery with interleaved shared callbacks — execute per-method with gating in a focused session, NOT a block-delete.

### TEAMS backend strip — ✅ DONE (session 58, `0b...` see git: "strip dead Teams backend")
Executed per the exact map (5 KEEP-traps all honored). Removed 15 dead TeamClient CRUD methods (team.rs 727→109) + ~18 UserWorkspaces CRUD wrappers/callbacks + team_client field + ctor params + lib.rs arg + CreateTeamResponse + ~22 graphql mutation imports + ~20 test-file mock args. KEPT: workspaces_metadata, TeamUpdateManager (workspace polling), team read-accessors, shared callbacks, stripe-billing. **3-gate 0/0/0, warnings 106/87/106 = baseline, warp_cli clean, −3.05 MB → 692.3 MB. ~1000 LoC.** No wrong-picture — the 5 traps were caught in mapping (workspaces_metadata / TeamUpdateManager-misnamed / read-accessors / stripe-billing-live / shared-callbacks).

### Referrals server fetch — ✅ DONE (session 58, "strip dead Referrals server fetch")
ReferralsClient (get_referral_info/send_invite) was dead-at-runtime (query_referral_status early-returns when not logged in). Deleted referral.rs + get_referrals_client + the fetch methods/wiring. KEPT ReferralThemeStatus (persisted theme-unlock survives) + theme_chooser. −0.59 MB → 691.7 MB. 3-gate 0/0/0, baseline warnings.

---

## BLOCK-SHARING strip — EXACT map (session 58; BlockClient is LIVE UI, not a dead strip)

BlockClient feeds the Warp-cloud **block-sharing** feature (share a terminal block to cloud / view+unshare shared blocks). LIVE UI, ~2,526 LoC. Real strip target per fork goal, but a MEDIUM touching pane_group (spider file). Surface:
- **Delete files:** `terminal/share_block_modal.rs` (1536 — ShareBlockModal/ShareBlockModalEvent/ShareBlockType), `settings_view/show_blocks_view.rs` (805 — ShowBlocksView/ShowBlocksEvent), `server_api/block.rs` (185 — BlockClient trait+impl: unshare_block/save_block/blocks_owned_by_user/generate_shared_block_title + Block type + ShareBlock/etc graphql ops).
- **pane_group/mod.rs:** `share_block_modal` field (733) + `terminal_with_open_share_block_modal` (728) + ShareBlockModal creation (1909) + handle_share_block_modal_event (2328) + ShareBlockModalEvent import/handling + init (1969/1970).
- **terminal/view.rs:** `OpenShareBlockModal` action (1114) + handler + the block-context-menu "Share Block" action that dispatches it.
- **pane_group/pane/terminal_pane.rs:404-405:** the open-modal trigger.
- **settings_view/mod.rs:** `SettingsSection::SharedBlocks` (178/195/244) + ShowBlocksView creation (925) + nav item (1023) + macro arm (855); **settings_page.rs:** `SharedBlocks(ViewHandle<ShowBlocksView>)` variant (98/114).
- **server_api.rs:** get_block_client (1174) + `use block::BlockClient` (21) + `pub mod block`.
- **terminal/mod.rs:93** re-export.
- Cascade: Block type, ShareBlockType, graphql ops (ShareBlock/BlockInput/etc).
**Execution:** leaf-up — settings (ShowBlocksView+SharedBlocks) → ShareBlockModal+pane_group/terminal_pane/terminal_view wiring → BlockClient/block.rs. Green per increment. Expect real shrink (2 views via add_typed_action_view + graphql ops, live-linked). ~2500 LoC, pane_group render-path — careful, own focused effort.

**BLOCK-SHARING — expanded surface (found during inc-1 attempt; settings is NOT an independent leaf):** removing `SettingsSection::SharedBlocks` cascades to a CustomAction + menu + workspace + modal chain:
- `util/bindings.rs`: `CustomAction::ViewSharedBlocks` variant (85) + match arm (428).
- `app_menus.rs:549`: the "View Shared Blocks" menu item (CustomAction::ViewSharedBlocks).
- `workspace/mod.rs:1154/1158`: `WorkspaceAction::ShowSettingsPage(SettingsSection::SharedBlocks)` + `.with_custom_action(CustomAction::ViewSharedBlocks)`.
- `share_block_modal.rs:759`: the modal's "view shared blocks" link → ShowSettingsPage(SharedBlocks).
- settings_view/mod.rs ServerApiProvider import becomes unused after get_block_client removal.
**Conclusion:** block-sharing is ONE coupled feature spanning settings + CustomAction/menu/keybinding + workspace + ShareBlockModal + pane_group + terminal/view + BlockClient (~2500+ LoC). Must strip as ONE operation (no independent leaf). Execution order: app_menus/bindings/workspace CustomAction::ViewSharedBlocks → settings SharedBlocks page → ShareBlockModal + pane_group/terminal_pane/terminal_view wiring → BlockClient/block.rs. Reverted inc-1 attempt to green (`97f34744`); do the whole thing in a focused session.

---

## BEDROCK strip — EXACT full map (session 58; user: "no Bedrock anything"). LARGE cross-crate + spider-file. Dedicated effort.

Bedrock = Warp's AWS-Bedrock LLM-provider integration (cloud-managed AWS STS identity token mint + login banners + workspace policy). ~15 app files + 6 crates.

**CORRECTION (session 59 re-map — the claim below was WRONG):** ~~`ManagedSecretManager` is Bedrock-ONLY~~. FALSE. `ManagedSecretManager` is GENERIC: `list_secrets()` + `get_task_secrets()` feed MCP/agent task-secret injection (Anthropic BYOK, OpenAI, raw values). Only its `issue_task_identity_token()` method is Bedrock-specific (AWS STS). **KEEP** the whole manager + `ManagedSecretsClient` + `server_api/managed_secrets.rs` + `get_managed_secrets_client` + lib.rs registration + `ManagedSecretValue` type (3 generic variants). **REMOVE** only the Bedrock bits: `issue_task_identity_token` call path, the 2 Bedrock `ManagedSecretValue` variants (AnthropicBedrockAccessKey/ApiKey) + constructors, and everything in the Bedrock surface below. Backend cut is much smaller than the old map assumed.

**App backend:**
- `ai/aws_credentials.rs` (whole — AwsCredentialRefresher, issue_task_identity_token OIDC, AwsCredentialsState) + aws_credentials_tests.rs.
- `server_api/managed_secrets.rs` (ManagedSecretsClient impl, 319) + server_api.rs get_managed_secrets_client + import.
- lib.rs ManagedSecretManager reg (1170) + import (188).
- terminal_manager.rs AwsCredentialRefresher use (29).

**App banner UI (spider-file render-path — terminal/view.rs, 25 refs):**
- inline_banner/aws_bedrock_login.rs (whole) + aws_cli_not_installed.rs (whole) + session_state.rs Bedrock bits + inline_banner/mod.rs exports.
- terminal/view.rs: InlineBannerType::AwsBedrockLogin (860/870), aws_bedrock_login_banner field (924), render (13902), remove_aws_bedrock_login_banner (5695), handle_aws_bedrock_login_banner_action (5705), run-aws-login (5731), AwsBedrockCredentialsEnabled subscribe (2875), BannerAction wiring (15846/16408), imports (74/78).
- terminal/view/action.rs Bedrock banner actions.

**App settings/workspace/debug:**
- settings/ai.rs: aws_bedrock_credentials_enabled + aws_bedrock_auto_login + aws_bedrock_auth_refresh_command + aws_bedrock_login_banner_dismissed (+ AwsBedrockCredentialsEnabled changed-event).
- workspaces/workspace.rs `LLMModelHost::AwsBedrock` variant (749) + user_workspaces.rs aws_bedrock_host_settings/is_aws_bedrock_available_from_workspace/host_enablement/credentials_toggleable/is_aws_bedrock_credentials_enabled (491-516) + gql_convert.rs:700 + user_workspaces_tests (230/266/320).
- workspace action/view/mod: DebugResetAwsBedrockLoginBannerDismissed (action.rs:480, view.rs:15727, mod.rs:159).

**Cross-crate (CAUTION):**
- crates/ai: api_keys.rs (3) + aws_credentials.rs (2) — Bedrock cred types.
- crates/graphql: api/queries/task_secrets.rs (5) + managed_secrets.rs (4) + get_feature_model_choices.rs AwsBedrock (72) + api/workspace.rs AwsBedrock (287) — graphql Bedrock ops + LLM-host enum variant.
- crates/managed_secrets: manager.rs (5, Bedrock-specific) — but secret_value.rs (18) is MIXED (ManagedSecretValue is MCP-used → KEEP the type, trim Bedrock-only bits). The whole `ManagedSecretManager`/client trait may be deletable if Bedrock is its only consumer (verify within crate).
- crates/warp_cli: secret.rs (17) + agent.rs (9) — cross-BINARY Bedrock/secret handling (run `cargo check -p warp_cli`).
- `LLMModelHost::AwsBedrock` enum-variant cascade: workspace.rs + 2 graphql enums (get_feature_model_choices, workspace) + gql_convert + tests — remove variant + all match arms.

**Execution (dedicated session, green per increment) — REVISED ORDER (session 59; settings are READ by the backend, so backend must precede settings):**
- (1) ✅ DONE `490b37aa` — banner UI: deleted inline_banner/aws_bedrock_login.rs + aws_cli_not_installed.rs + InlineBannerType variants + InlineBannersState fields + is_pending_aws_login + AISettings subscription + 6 banner methods + run_aws_login PTY path + block-completed caller + render + TerminalAction variants + dispatch. 3-gate 0/0/0, 104/85/104. KEPT ByoLlmAuthBannerSessionState (singleton registered in lib.rs — defer with its cascade).
- (2) ✅ DONE `bdaff286` + dep-drop `6eab8235` — BACKEND: deleted app/src/ai/aws_credentials.rs + crates/ai/src/aws_credentials.rs + tests + the ApiKeyManager AWS surface + lib.rs/terminal_manager wiring; then dropped the 4 orphaned aws-sdk Cargo deps. **−25.46 MB** (aws-sdk-sts + aws-config transitive trees were live-linked — the big win). **CORRECTION:** `ManagedSecretManager::issue_task_identity_token` is GENERIC, NOT Bedrock-only — manager.rs:151 calls it inside `get_task_secrets` (MCP path). KEPT it + the graphql op + ManagedSecretsClient trait method.
- (3) ✅ DONE `ac9c9eee` — SETTINGS + WORKSPACE: deleted 4 settings (credentials_enabled/auto_login/auth_refresh_command/profile) + LLMModelHost::AwsBedrock + the 5 user_workspaces accessors + gql_convert (now maps wire AwsBedrock→Unknown) + 3 Bedrock test fns. **CORRECTION:** the crates/graphql `LlmModelHost::AwsBedrock` variants CANNOT be removed — cynic validates the enum against the live server schema (which has AWS_BEDROCK); leave the wire variant, collapse at the app boundary.
- (4) ✅ DONE `98e4aaff` — ByoLlm/DebugReset: deleted ByoLlmAuthBannerSessionState (session_state.rs) + registrations + the aws_bedrock_login_banner_dismissed setting + WorkspaceAction::DebugResetAwsBedrockLoginBannerDismissed. **App-side Bedrock now FULLY stripped.**
- (5) ✅ DONE (warp_cli part) `cb9a41e0` — warp_cli secret.rs Bedrock subcommands (BedrockApiKey/BedrockAccessKey + arg structs + SecretType::AnthropicBedrockApiKey) + agent.rs bedrock_inference_role/bedrock_role_region args + 3 lib_tests removed (all had ZERO consumers tree-wide — parsed-but-undispatched). warp_cli 0/0, warp default 0/104. **BEDROCK STRIP COMPLETE** except the schema-bound plumbing below.
- **KEPT (schema-bound, NOT removable):** managed_secrets `ManagedSecretValue::{AnthropicBedrockAccessKey,AnthropicBedrockApiKey}` variants + constructors + manager.rs:89-99 mapping arms; crates/graphql `ManagedSecretType::AnthropicBedrock*` + `task_secrets.rs ManagedSecretAnthropicBedrock*` union members. All cynic-validated against the live server schema (which still has these secret types) OR feed the generic `get_task_secrets` MCP download path. Removing them would force the manager to drop wire secret-types it's handed — ~0 binary, pure plumbing. Treat as KEEP (like the LLMModelHost::AwsBedrock wire variant). KEEP ManagedSecretValue type + ManagedSecretManager + issue_task_identity_token (all generic/MCP).

---

# SPIDER-FILE PASS — plan (the final big effort; Step 8 of the removal order)

The remaining cloud/AI lives woven into the core render/state "spider" files. This is the explicitly-LAST step: excise AI/cloud branches while PRESERVING render + the kept CLI-agent feature. Do as a dedicated session (or several), green per increment.

## Files + surface (measured session 58)
| file | LoC | AI/cloud refs* |
|---|---|---|
| `terminal/view.rs` | 17,302 | ~565 |
| `workspace/view.rs` | 17,757 | ~428 |
| `terminal/input.rs` | 9,428 | ~437 |
| `pane_group/mod.rs` | 5,080 | ~96 |
| `root_view.rs` | 1,735 | few |
*raw grep `agent|conversation|cloud|share_block|bedrock|handoff|ambient|ai_` — OVERCOUNTS: most `agent` hits are the KEPT vendor-CLI-agent feature.

## CRITICAL keep/remove rule
- **KEEP:** vendor CLI agents (claude/codex/gemini/aider as PTY processes) — `cli_agent`, `cli_agent_sessions`, `cli_controller`, plugin_manager, the CLI-agent footer/rich-input, AgentReviewCommentBatch/code-review feed, `conversation_status_ui`/agent_icon (CLI status), `ConversationStatus`, `AIConversationId`/`ServerConversationToken` (persistence ids), MCP, `ManagedSecretValue`, generic terminal blocks/render.
- **REMOVE:** Warp-AI/cloud branches — block-sharing, Bedrock banners, remaining ambient-agent/cloud-handoff hooks, dead AI-action/context-menu variants, any `is_logged_in`/cloud-gated branches now always-false.
- Distinguish by NAME + caller: "agent" alone is usually CLI (keep); "ambient_agent"/"cloud"/"handoff"/"bedrock"/"share_block"/Warp-AI-conversation = remove. Verify each (Teams lesson: names lie, e.g. TeamUpdateManager polled workspaces).

## Sub-strips (each has a recorded EXACT map above)
1. **block-sharing** ✅ DONE — stripped as one coupled op. Deleted: `terminal/share_block_modal.rs` (1536), `settings_view/show_blocks_view.rs` (805), `server/server_api/block.rs` (BlockClient, 185), `server/block.rs` (server Block repr, dead after BlockClient), `ai/generate_block_title/` (dead Request/Response types), 3 graphql ops (share_block/unshare_block/get_blocks_for_user) + mod decls. Excised: ContextMenuAction::OpenShareBlockModal + TerminalAction::OpenShareModal + Event::ShareModalOpened + the 2 view.rs handler fns + dispatch arms; CustomAction::ViewSharedBlocks (variant + 2 match arms + app_menu item + workspace binding); SettingsSection::SharedBlocks + SettingsPageViewHandle::SharedBlocks (Display/FromString/macro/nav/should_render/page-build); pane_group share_block_modal+terminal_with_open_share_block_modal fields + handler + cleanup + render + ctor; terminal_pane.rs ShareModalOpened arm; init.rs keybinding + resource_center keybinding-name; dead `full_content_height_with_display_options` block-height helper. **3-gate 0/0/0, warnings 106/87/106 = baseline, warp_cli clean.** TRAP avoided: there were TWO `block.rs` (server_api/block.rs = BlockClient; server/block.rs = the server-side Block struct + DisplaySetting) — investigator only found the first; the second + its lone consumer `terminal/model/block.rs::full_content_height_with_display_options` were dead and removed too.
2. **Bedrock** (see "BEDROCK strip — EXACT full map") — banner UI in terminal/view + backend + settings + workspace LLMModelHost::AwsBedrock + 6 crates incl warp_cli. Largest; cross-crate. ManagedSecretManager Bedrock-only (cascades; keep ManagedSecretValue).
3. **Residual AI handlers/branches** in terminal/view.rs + input.rs — 🔄 LARGELY DONE (session 59, `6b10c241` 1/2 + `5cbf1e16` 2/2, warnings 106→104). Removed ~30 dead methods/fields: orchestration-split-off cluster, conversation/plan stubs (attach_plan_as_context, is_conversation_selected, active_conversation_task_id, stop_local_agent_conversation, remove_pending_user_query_block), create_new_project/agent_clone_repository + initiate_clone_repository, execute_command_or_set_pending, has_pending_command_or_awaiting_completion, ContextMenuInfo+telemetry helpers, PromptSuggestion coding-query fields, open_plan_menu, remove_excess_images, unfreeze_and_clear_agent_input, the submit_ai_query→ExecuteAIQuery cascade (+ abort_attached_images_future_handle), last_intelligent_autosuggestion_result+IntelligentAutosuggestionResult, pane_group close_panes + insert_terminal_pane_hidden_for_child_agent/attach_child_pane_off_tree child-agent plumbing, blocks.rs removable_blocklist_item_position. **Follow-ups:** (i) `attachment_chips` dead cluster ✅ DONE `17dc11f7` (Input field + AttachmentChip + 2 render fns + TerminalAction::DeleteAttachment + 8 orphaned imports; 104→103); (iii) `submit_queued_prompt` + its 2 slash_command_model_tests ✅ DONE `d55b7872`. **Spider-file cleanup of bounded dead items = COMPLETE.** Remaining are DEDICATED efforts, not residual cleanup: (ii) `server_api` Input field — "fix Input struct" found STALE/done in S60 (ai_context/action_model already 0 refs; ai_input_model/cli_subagent_controller are no-op stubs not Input fields); (iv) **cloud-handoff** — increment 1 ✅ DONE (S60, `03fbe7b3`+`b9cf1b24`: dead auto-cloud-handoff subsystem `workspace/auto_handoff.rs` + SystemStats sleep/wake cascade). REMAINING inc 2/3: manual handoff (`OpenLocalToCloudHandoffPane` slash cmd) + handoff-snapshot RPC → `ai-strip-session-60-cloud-handoff.md`. KEEP handoff_compose (`&`-prefix) + cross_window_tab_drag (generic); (v) `active_conversation_id` (29 refs, keystone — defer with conversation-id plumbing).

## Technique (proven this strip)
- Exact-map-first per sub-strip (investigator: file:line + live-vs-dead + KEEP/REMOVE per symbol). NO blind cuts — the spider files are where wrong-picture cuts break render.
- Collapse flag-gated branches first (flag stays → green), drop the branch when readers=0. Cargo-feature-gated AI (like voice) → drop feature, sweep cfg sites.
- Per-sub-strip green commit; 3-gate + `-p warp_cli` + GUI build each. Revert-to-green if a sub-strip proves coupled mid-cut (block-sharing inc-1 lesson).
- Enum-variant cascades (ContextMenuAction/TerminalAction/WorkspaceAction AI variants, LLMModelHost::AwsBedrock): remove variant + ALL match arms together; compiler enumerates break sites.

## Suggested order (least → most coupled)
(a) block-sharing ✅ DONE (`cecfc791`, −2.20 MB, session 59) → (b) residual dead AI handlers (investigator-found) → (c) Bedrock (app banners → settings → workspace enum → backend → crates → warp_cli). Each green. Expect the real binary payoff here (banners/modals/OIDC/graphql ops are live-linked through the spider files).

## Risk
Spider files are render/state core — a bad cut breaks the GUI silently (compiles, mis-renders). After each sub-strip: 3-gate + `cargo build --bin sublight --features gui` + launch-smoke if possible. This is why it's LAST and dedicated.
