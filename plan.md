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
| Telemetry | ⬜ todo | — | Category 1, mandated (all telemetry). |
| firebase + experiments + wasm crates | ⬜ todo | — | EASY crate deletions. |

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

## 1. Telemetry — REMOVE ALL (user mandate)

| Finding | Location | Size | Difficulty | Why |
|---|---|---|---|---|
| `send_telemetry*` macros | `app/src/server/telemetry/macros.rs` | 5 macros | **EASY** | No-op these FIRST → all 795 call-sites compile to nothing, tree stays green. |
| Telemetry call-sites | 167 files across `app/src` | **795 calls** | **MEDIUM** | Mechanical once macros are no-op'd; some sit in render files. Delete incrementally. |
| `TelemetryEvent` enum | `app/src/server/telemetry/events.rs` | 767 variants, 7.3k LoC | **MEDIUM** | Pure data enum; ~196-file fan-in via match arms. Delete after call-sites gone. |
| Collector + Rudderstack dispatch | `server/telemetry/{collector,mod,rudder_message,context,secret_redaction}.rs` | ~3.7k LoC | **EASY** | Self-contained sender (HTTP → Rudderstack). |
| Crash reporting / Sentry | `app/src/crash_reporting/` (4 files) | 1.2k LoC | **EASY** | Feature-gated (`crash_reporting`/`cocoa_sentry`), NOT in default. Flip off + delete. |
| warpui_core telemetry | `crates/warpui_core/src/telemetry/` + `app_focus_telemetry.rs` | ~555 LoC | **MEDIUM** | Backs the macros (event queue, focus tracking). Remove after call-sites. |
| warp_core telemetry trait + RudderStackConfig | `crates/warp_core/src/{telemetry.rs,channel/config.rs}` | ~380 LoC | **EASY** | Endpoint/write-key config + trait. |
| Analytics feature flags | `app/src/features.rs` | 4 flags | **EASY** | `GlobalAIAnalyticsCollection`, `AgentModeAnalytics`, `RecordAppActiveEvents`, `WithSandboxTelemetry`. |
| Profiling (pprof/dhat) | `app/src/profiling.rs` | 121 LoC | **EASY** | Optional heap/CPU upload, self-contained. |
| Telemetry bootstrap wiring | `app/src/lib.rs` ~773–1555 | scattered | **HARD** | Mixed with auth/db init; final wiring step. |

**Approach**: no-op the macros → green → delete the now-dead call-sites in batches → delete the infra (collector/dispatch/event enum) → strip bootstrap wiring last. ~1188 total touch-points but the no-op-macro trick collapses most risk.

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
10. **Scrub** — dead feature flags, dormant config, grep for phone-home (warp.dev/firebase/rudderstack). Build `warp-oss --features gui`, launch, verify render.

---

## Resume notes (post-compaction continuation)

**Where**: worktree `~/Documents/git/warp-upstream`, branch `surgical-strip` off upstream `0b737e22` (green baseline). `strip-cloud` (old delete-foundation-first attempt) is abandoned — ignore its `AGENTS.md`; this `plan.md` + `docs/superpowers/specs/2026-05-27-surgical-cloud-strip-design.md` are the living docs.

**Verify green** (background it — slow):
- build/launch path: `cargo check -p warp`
- tests: `cargo check -p warp --tests`
Both must be 0 errors before each commit.

**Build + launch the GUI**: `cargo run --bin warp-oss --features gui` (NOT `./script/bootstrap` — Debian/apt-only; CachyOS deps already present). First `--features gui` build is long. Confirmed: app launches straight to a terminal (no welcome/onboarding/login).

**Done + verified**: welcome panes (`59562bd7`), onboarding flow (`3e87396f`). Login KEPT.

### Cleanup TODO (from onboarding pass — non-blocking, build+tests green)
- ✅ Onboarding action stubs + dead variants/enums + helper methods removed (`d444c2b6`). `ImportSettings` was NOT a stub to delete — it was a mis-stubbed local feature, restored (`e9a3dc2e`).
- **Remaining 79 warnings — DEFERRED to subsystem passes (not blindly swept).** They are orphaned unused imports + dead fns left by the onboarding `AgentOnboardingEvent` handler removal, but they belong to other subsystems and are cleanest removed *with* those subsystems (advisor guidance — avoids feature-gate false-positives + keeps per-pass attribution):
  - Login pass: `login_slide.rs` (`VISUAL_IMAGE_PATHS`, `resolve_visual_path`, `is_auth_token_input_visible`/`new`/`handle_auth_manager_event`, `LoginSlideSource` variants), `paste_auth_token_modal::new`, root_view imports `LoginSlide*`/`PasteAuthTokenModalEvent`.
  - AI pass: `LLMPreferences`/`refresh_*_models`, `HistoryEvent`, `apply_natural_language_detection_setting`'s old setting (`AISettings`), `EntrypointType`/`StaticQueryType`.
  - Cloud/billing pass: `CloudPreferencesSyncer*`, `PricingInfoModelEvent`, `build_plan_yearly_price_cents`, `upgrade_url`.
  - Experiments pass: `is_free_user_no_ai_experiment_active`.
  - Teams pass: `TeamUpdateManager`, `UserWorkspaces*`.
  - Generic orphans (could sweep anytime, low value): root_view `ClipboardContent`/`report_if_error`/`TabSettings`/`ThemeKind`/`WarpThemeConfig`/`AISettings`/`ThemeSettings`; terminal/view `TerminalKeybindings`.
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
