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
| Orphaned get-started sub-views (`coding_entrypoints/`: `clone_repo_view`, `create_project_view`, `project_buttons`) | ⬜ cleanup todo | — | Dead since `get_started_view` deleted (dead_code warnings, build still green). `project_buttons::init` still called in `lib.rs:1605`. Remove module + init call in a follow-up. |
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
| Cloud preferences syncer | `app/src/settings/cloud_preferences*.rs` | 2 files, ~1.1k LoC | **MEDIUM** | Settings→cloud sync; 19 fan-in. |
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
| Resource center | `app/src/resource_center/` | 35 files, 2.2k LoC | **MEDIUM** | Menu subsystem; 35 fan-in. |
| Get-started landing tab | `pane_group/pane/get_started*.rs` | 484 LoC | **MEDIUM** | Sibling landing surface (`get_started_tab` feature). Same as welcome screen — remove → new tab defaults to terminal pane. Feature-gated; 21 refs. |
| Referrals | `referral_theme_status.rs` + `referrals_page.rs` | ~400 LoC | **MEDIUM** | Settings subsection; auth-coupled. |
| Bonus grant notification | `workspace/bonus_grant_notification_model.rs` | 132 LoC | **MEDIUM** | Billing-coupled. |
| Onboarding crate | `crates/onboarding/` | 36 files, 11.5k LoC | **MEDIUM** | ⚠ Depended on by `ai` crate — partly blocked by AI removal. Feature-gated. |
| Onboarding UI (block + HOA) | `terminal/view/block_onboarding/` (1.6k) + `workspace/hoa_onboarding/` (1.1k) | ~2.7k LoC | **MEDIUM** | ~104 refs in `terminal/view.rs`, modular. |
| Agent tips | `app/src/ai/agent_tips.rs` | 673 LoC | **MEDIUM** | Feature-gated; AI-coupled. |
| Warpify footer/banner | `app/src/terminal/warpify/` | 53 files, 1.8k LoC | **HARD** | 155 refs in `terminal/view.rs` render path. |
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

### AI strip — current state (`0d24336f`, 2026-06-01 session 4)

**Binary**: 794.3 MB (unchanged — handler bodies were already dead-stripped). 3-gate **71/86/71** (better than pre-existing 72/87/72; `is_conversation_selected` stub added for workspace caller at `7a710de6`). Commits `0d24336f` + `7a710de6`.

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
