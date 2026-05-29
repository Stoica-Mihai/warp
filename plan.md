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
| firebase crate | ⏸ deferred → login pass | — | Two callers: `server_api/auth.rs` (login token fetch) + `sync_queue_tests.rs`. Touching `auth.rs` to drop firebase = doing the login backend refactor. Plan.md §104 confirms: "removes cleanly *as part of* removing the auth/server layer, not standalone." |
| Experiments (A/B) `app/src/server/experiments/` | ⏸ deferred → separate investigation | — | 7 callers, including `persistence/sqlite.rs` (KEPT for session restore) and `workspaces/`. `ServerExperiment` may be a persisted session-column type — deletable only after mapping the persistence shape. Not a clean "EASY crate deletion." |
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
| `ai_assistant` panel | `app/src/ai_assistant/` | 10 files, 3.6k LoC | **EASY** | Self-contained AI panel, feature-gatable. |
| `ai` engine crate | `crates/ai/` | 75 files, 25k LoC | **MEDIUM** | Fairly self-contained, but 628 app callers — delete only after callers refactored. |
| Execution profiles + model selector | `app/src/ai/execution_profiles/`, `terminal/profile_model_selector.rs` (2.4k) | 9 files | **MEDIUM** | LLM model picker; 56-file fan-in. |
| AI context menu / context chips | `app/src/context_chips/` | 23 files, 11.7k LoC | **MEDIUM** | Mixed into terminal input UI. |
| AI settings pages | `settings_view/{ai_page,execution_profile_view}.rs` | ~9.4k LoC | **MEDIUM** | Profile/LLM settings. |
| `app/src/ai/agent` (LLM exec core) | `app/src/ai/agent/` | 32 files, 22.5k LoC | **HARD** | Warp agent driver/harness/todos/SDK. |
| `app/src/ai/blocklist` (AI block render) | `app/src/ai/blocklist/` | 179 files, 102k LoC | **HARD** | AI block rendering + interaction; 296-file fan-in. |
| Conversation/history models | `app/src/ai/blocklist/history_model.rs`, `agent_conversations_model.rs` | ~15k LoC | **HARD** | AI chat state, conversation IDs; on-disk. |
| Render-path AI coupling | `terminal/view.rs` (203 refs), `input.rs` (151), `pane_group/mod.rs` (95), `workspace/view.rs` (105) | — | **HARD** | Core spider files — excise AI branches, keep render. Do LAST. |

**Total**: `app/src/ai` 453 files / 217k LoC + `crates/ai` 75 files / 25k LoC. Hotspot: `terminal/view.rs`.

---

## 3. Warp proprietary — needs Warp's backend (login/cloud/sharing/teams/billing)

| Finding | Location | Size | Difficulty | Why |
|---|---|---|---|---|
| **firebase crate** | `crates/firebase/` | 1 file, 145 LoC | **EASY** | ✅ Confirmed deletable. Only `server/server_api/auth.rs` + 1 test reference it. |
| Experiments (A/B) | `app/src/server/experiments/` | 4 files, 521 LoC | **EASY** | Server experiment flags. |
| Cloud network crates | `graphql`, `warp_server_client`, `warp_graphql_schema`, `websocket`, `managed_secrets`, `warp_web_event_bus` | ~6 crates | **EASY*** | Pure API layers — *but `websocket`/`graphql` pulled by `warp_core`/`ai`, so blocked until those callers go. `warp_web_event_bus` = wasm-only, removable now. |
| Cloud preferences syncer | `app/src/settings/cloud_preferences*.rs` | 2 files, ~1.1k LoC | **MEDIUM** | Settings→cloud sync; 19 fan-in. |
| Teams / billing / API keys | `settings_view/{teams_page,billing_and_usage*}`, `app/src/billing/` | ~4.4k LoC | **MEDIUM** | Feature-gated pages. |
| Shared sessions | `app/src/terminal/shared_session/` | 39 files, 5.1k LoC | **MEDIUM** | Session relay UI/logic. |
| Auth / login UI | `app/src/auth/` | 22 files, 7.8k LoC | **MEDIUM** | Firebase token + login UI; 174 fan-in. |
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

### shared_session strip — IN PROGRESS (session-sharing cloud feature)

**Goal**: remove Warp's terminal session-sharing (share live session over Warp cloud; join/view via link). NOT a generic AI capability — vendor CLIs never touch it.

**Two homes + core type**:
- `terminal/shared_session/` (dir, ~14.8k LoC) — engine: manager, network, sharer, viewer, presence_manager, permissions_manager, participant_avatar_view, render_util, replay_agent_conversations, role_change_modal, share_modal, shared_handlers, selections, settings, ai_agent.
- `terminal/view/shared_session/` (dir, ~6.7k LoC) — view layer: adapter, view_impl, conversation_ended_tombstone_view, cloud_conversation_continuation, sharer/viewer view. Depends on dir #1.
- **`SharedSessionStatus`** (in `terminal/shared_session/mod.rs:99`) — the sharer/viewer/reader/executor permission state ON `terminal_model`. ~11 predicates (`is_viewer`/`is_sharer`/`is_reader`/`is_executor`/`is_active_sharer`/…). Read at **111 sites**, heaviest in `terminal/input.rs` + `terminal/view.rs`.

**Done**: ✅ commit 1 `144639a4` — AI agent_sdk + `warp_cli::share` decouple (ShareSessionError, should_share, wait_for_session_shared, add_share_requests, EstablishedSharedSession event, write_session_joined, --share flag). 3-gate green, 0 warnings. terminal/shared_session still mounted.

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
