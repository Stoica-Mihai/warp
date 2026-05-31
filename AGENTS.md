# AGENTS.md

Operating manual for any agent (or human) picking up this fork. (`CLAUDE.md` is a symlink to this file so it auto-loads.)

## 1. What this is

Personal fork of [warpdotdev/warp](https://github.com/warpdotdev/warp), stripping Warp's hosted-backend coupling (auth/login, AI agent panel, billing, cloud sync / Warp Drive, telemetry, session-sharing, teams, Oz). **Goal: launch Warp's GUI with no cloud features.** Same window, terminal blocks, settings, themes, MCP, LSP, vim — minus the cloud surfaces. Vendor agent CLIs (claude, codex, gemini, aider) still run as ordinary shell processes inside.

## 2. Approach (surgical, green-always)

- **Branch `surgical-strip`, forked from the last green upstream commit `0b737e22`** (worktree: `~/Documents/git/warp-upstream`). This replaced an earlier `strip-cloud` branch that deleted the `ai` crate before its callers and never compiled — that approach is abandoned.
- **Invariant: `cargo check -p warp` = 0 errors after every commit.** Remove cloud feature-by-feature; never delete a dependency before refactoring its callers off it. Also keep `cargo check -p warp --tests` green.
- Removal loop: delete the def (enum variant / module / method), let `cargo check` enumerate every break site, fix in batches, repeat to 0.
- **Gate matrix (3 runs)**: `cargo check -p warp` (default) + `--tests` + `--features local_fs,gui` (combined). The combined feature run covers both feature gates simultaneously — features compose, so no need for separate `--features gui` and `--features local_fs` passes.

## 3. Status, plan, lessons → `plan.md`

**`plan.md` (this directory) is the single source of truth** for status, per-finding difficulty, removal order, resume notes, and lessons. Read it first. Strategy detail: `docs/superpowers/specs/2026-05-27-surgical-cloud-strip-design.md`. Build-size trend across the strip: `build-size-log.md` — add a row after every meaningful strip lands. Keep §1–§2 here synced with `plan.md`; let `plan.md` hold everything volatile.

Done so far — full per-commit log in `git log` and `plan.md`. Areas complete (all 3-gate green):

telemetry · wasm-orphan crates · onboarding crate + flags · Sentry crash-reporting · Sublight rebrand · channel enum cascade · autoupdate pipeline · session-sharing (all vestiges incl. `SharedSessionStatus` + `is_shared_session_viewer` cascade) · Warp Drive server-sync + SyncQueue + ObjectClient · SharingDialog + permission-CRUD · login gate bypass · auth gate UI · AuthManager login stub · Firebase crate · server-driven A/B experiments · dead AuthClient privacy-sync · AuthView/AuthOverrideWarningModal cloud-gate UI · billing/teams/platform/referrals pages · AI assistant panel + Warp AI command search · AI settings pages + execution profile editor · execution profiles data model + inline profile selector · ScheduledAgentManager registration + AgentSource computation · **Phase C: `ai/agent_sdk/` STRIP COMPLETE** (`d3eb301d` + `889d83dd`) · **Phase E+F complete: `ai/blocklist/agent_view/` deleted + AI view files** (`94c6be3f`) · `TextLocation` relocated to `util/text_location` + `LinkActionConstructors` to `util/link_detection` (`73dd201c`)

**Current state:** Binary **813.2 MB** (−31.0 MB total vs 844.2 MB baseline). **0 errors / ~250 warnings** (all AI territory, deferred, no `#[allow(dead_code)]` cheats). App always starts in Terminal.

**Phase E+F done** (`94c6be3f`, −7.80 MB):
- `ai/blocklist/agent_view/` deleted (entire dir, all agent view UI)
- `terminal/view/agent_view.rs`, `load_ai_conversation.rs`, `use_agent_footer/`, `pending_user_query.rs` deleted
- `terminal/input/agent.rs`, `terminal/input/conversations/` deleted
- `ai/active_agent_views_model.rs` deleted
- Non-AI types relocated: `AgentToolbarItemKind` → `context_chips/toolbar.rs`; `AgentViewState/EntryOrigin/DisplayMode/render_block_container` → `terminal/view/agent_view_state.rs`; `AgentInputButtonTheme` → `terminal/view/ambient_agent/button_theme.rs`; `ConversationRestorationInNewPaneType` inlined into `terminal/view.rs`
- `ActiveAgentViewsModel` singleton removed from lib.rs (14 callers fixed)
- All agent-view fields stripped from `TerminalView`, `Input`, `workspace/view.rs`, `pane_group/pane/terminal_pane.rs`

**Next: Phase F+G (must do together)** — correct deletion order:

1. **First excise `ai_controller`/AIBlock from `terminal/view.rs` (369 refs) and `terminal/input.rs`** — remove `ai_controller: ModelHandle<BlocklistAIController>` field, AIBlock construction/subscription/rendering branches, and all `BlocklistAI*` model references from the terminal view. This is Phase G-preview and is the prerequisite.

   **SCOPE WARNING (2026-05-31 hard lesson):** Phase G-preview removes `ai_controller` + direct dependents (`passive_suggestions_models`, `cli_subagent_controller`, `ai_render_context`, `agent_todos_popup`, `get_relevant_files_controller`, `cli_subagent_views`). Do **NOT** also remove `ai_input_model`, `ai_context_model`, `ai_action_model` from `terminal/input.rs` Input struct — those fields are used by 50+ method bodies in Input and its render subfiles (`classic.rs`, `universal.rs`, `terminal.rs`). Removing them causes 150+ cascade errors that require Phase F level surgery. Leave those 3 in Input until Phase F deletes the model files that define them.

   **Verified recast_structural query for 28 AI-only terminal/view.rs methods (run with `include_leading_attrs: true`, `template: ""`, `max_bytes: 15000000`):**
   ```
   (function_item name: (identifier) @name (#match? @name "^(handle_ai_controller_event|handle_legacy_passive_suggestions_event|handle_ai_context_model_event|handle_ai_history_model_event|handle_cli_subagent_controller_event|handle_resume_conversation|handle_usage_footer_toggled|handle_ai_input_model_event|handle_ai_action_model_event|focus_ai_block_if_self_focused|handle_maa_passive_suggestions_event|handle_ai_block_event|active_ai_block|last_ai_block|handle_shell_command_executor_event|handle_start_agent_executor_event|get_ai_notification_summary|maybe_insert_tombstone_for_non_running_shared_ambient_task|on_next_conversation_finished|update_context_blocks_and_exchanges|drop_hidden_passive_ai_blocks|clear_prompt_suggestions|try_clear_prompt_suggestions_banner_code_state|remove_pending_cloud_mode_query_if_exchange_has_renderable_user_query|pending_user_query_conversation_id|remove_pending_user_query_block|stop_local_agent_conversation|apply_cli_agent_footer_visibility)$")) @fn
   ```
   Second pass (7 more): `build_agent_todos_popup|handle_agent_todos_popup_event|ai_controller|ai_context_model|ai_input_model|active_conversation_id|active_conversation_task_id`

   **Input::new() param removals (minimal — keep ai_input_model, ai_context_model):** remove only `ai_controller`, `ai_action_model`, `cli_subagent_controller` from the signature.

2. **Then delete blocklist AI model layers** (safe once terminal/view.rs callers are gone):
   - `ai/blocklist/block.rs` + `block/` (except: relocate `secret_redaction` → `app/src/secret_redaction.rs` first; relocate `keyboard_navigable_buttons`, `toggleable_items`, `numbered_button`, `compact_agent_input`, `inline_action_header`, `inline_action_icons`, `requested_action`, `WithContentItemSpacing` to `terminal/view/` — these are used by `init_project/`, `init_environment/`, `ssh_remote_server_choice_view`, `ambient_agent/`)
   - `ai/blocklist/controller/` + `controller.rs`
   - `ai/blocklist/action_model/` + `action_model.rs`
   - `ai/blocklist/history_model.rs` + `history_model/`
   - `ai/blocklist/input_model.rs`
   - `ai/blocklist/context_model.rs`
   - `ai/blocklist/orchestration_events.rs` + `orchestration_topology.rs` + `orchestration_event_streamer.rs`
   - `ai/blocklist/task_status_sync_model.rs`
   - `ai/blocklist/permissions.rs`, `persistence.rs`, `passive_suggestions/`, leaf files
   - lib.rs: remove `BlocklistAIHistoryModel`, `BlocklistAIPermissions`, `OrchestrationEventService`, `TaskStatusSyncModel`, `OrchestrationEventStreamer`, `LocalSharedSessionLinkModel` registrations

**KEEP in blocklist/:** `prompt/` + `prompt.rs`, `view_util.rs`, `keystroke_render.rs`, `code_block.rs`. KEEP `ai/mcp/`.

**KEEP terminal/input/:** `inline_menu/`, `message_bar/`, `inline_history/`, `cloud_mode_v2_history_menu.rs` — these are **shared terminal UI infrastructure** used by 28+ non-AI modules (slash_commands, skills, plans, repos, rewind, models, etc.). NOT AI-only.

**AIBlock is Warp-only AI feature.** Vendor CLI agents (claude, codex, etc.) run as ordinary shell PTY processes — they never use AIBlock. Safe to delete entirely.

**CRITICAL LESSON:** Never bulk-remove imports without simultaneously removing the type uses in those files. Removing imports alone causes more errors than having the directory missing.

**Deferred:** `IsSharedSessionCreator` (used in child_agent.rs + pane_group + terminal_pane + terminal_manager + docker_sandbox via `SharedSessionSource`/`inherit_share_for_local_child`; remove with child_agent seam). `session_sharing_protocol` crate **KEEP** — `SharedSessionSource` + `SessionSourceType` load-bearing for ambient agents. `referral_theme_status.rs` **KEEP** — woven into theme_chooser + GlobalResourceHandles. `persisted_workspace.rs` **KEEP** — LSP workspace tracking, not cloud AI.

**Key techniques:** Collapse flag-gated `if` branches first (flag still defined → green), drop flag def last when readers = 0. Trace dead UI subsystem to its event emitter — if gated on always-false predicate, whole chain removes cleanly. Telemetry: no-op send-macros first → event enums delete without touching ~819 call-sites; `recast` regex non-greedy `(?s)NAME!\(.*?\);`. `cargo fix` for bulk unused removal but re-verify all 3 gates (drops `#[cfg(test)]` imports). After large multi-file sweep, run one gate cold or `cargo run` to bust incremental cache before trusting exit 0.

## 4. Build / verify

- Build + launch GUI: `cargo run --bin sublight --features gui`. Do **not** run `./script/bootstrap` (Debian/apt-only; on this CachyOS box the deps are already present). First `--features gui` build is long. Binary lands at `target/debug/sublight`.
- Verify green: 3-gate matrix — `cargo check -p warp` + `--tests` + `--features local_fs,gui`. **Run all 3 in parallel, each in its OWN target dir** — they share the same `target/` otherwise and cargo's build lock serializes them (one waits for the others, defeating the point). Give each its own `CARGO_TARGET_DIR` and launch them as background tasks in a single message:
  ```bash
  CARGO_TARGET_DIR=target/gate-default cargo check -p warp
  CARGO_TARGET_DIR=target/gate-tests   cargo check -p warp --tests
  CARGO_TARGET_DIR=target/gate-feat     cargo check -p warp --features local_fs,gui
  ```
  Run each via a separate backgrounded `Bash` call (`run_in_background: true`) in the same response, then collect results when notified. Wall-clock ≈ slowest single gate (~2m) instead of the sum (~5m). The per-gate dirs are first-build-cold but warm on reruns; they cost extra disk but are reusable across sessions (don't `cargo clean` them between checks). The GUI binary still builds into the main `target/` via `cargo run/build --bin sublight --features gui`.
- LSP works on this worktree, but injected diagnostics lag edits (stale line numbers). Trust `cargo check`, not the diagnostic stream.

## 4a. After every strip commit — mandatory size log step

**Do not skip this. Every strip commit must get a row in `build-size-log.md`.**

After the 3-gate matrix passes and you commit the refactor:

1. Run `cargo build --bin sublight --features gui` (background, `run_in_background: true`). Note: if the binary is already up-to-date cargo exits in <2s; if not, it takes ~2m. Either way, wait for the notification.
2. Read the binary size: `stat -c%s target/debug/sublight`
3. Compute delta vs the last row in `build-size-log.md` (bytes, MB SI, MiB).
4. Add a row to `build-size-log.md`. Include: date, commit hash, profile=debug, step description, bytes, MiB, MB, build time (from `time` output or "cached"), and a notes field explaining whether the delta is real or linker-invisible and why.
5. Commit: `docs: add build-size row for <hash> (<delta> <step>)`.

**Do NOT assume the delta is zero** — UI view deletions can be non-trivial (auth view strip was −2.27 MB because `TypedActionView` impls + `ctx.add_typed_action_view` registrations + `init()` calls were live-linked). Measure every time.

## 5. Conventions

Conventional commits (`refactor:`/`feat:`/`fix:`/`docs:`/`chore:`). No `Co-Authored-By` trailers. No `--no-verify`. Commit only at green checkpoints. Don't claim done on a stub — flag seams honestly.

## 6. Multi-file rewrites — use `recast` MCP tools

This removal loop is one shape repeated across many sites — exactly recast's win zone. The `recast` MCP server is wired globally; prefer its tools over `Edit`/`sed` loops when the same syntactic change lands in many files.

- **Find before you rewrite — use `recast_search`** instead of grep/Bash for locating symbols, usages, and callsites. Two modes: regex (`pattern`) or structural (`lang` + `ast_pattern`). Returns structured JSON `{files, matches: [{line, col, snippet, capture}]}`. Zero-match guard — silent misses impossible. Structural captures (`@name`) distinguish definitions from usages. Prefer over grep whenever structured output or AST-aware matching is needed.
- **5+ sites, simple text change** (rename, drop a feature flag, swap an import): `recast_preview` → inspect diff → `recast_apply`. 1-4 isolated sites: `Edit` is fine.
- **Shape-sensitive change** (remove an enum variant, add/drop a struct field, change a fn signature across callers): `recast_structural` with `ast_pattern` (`lang: "rust"`). Regex on AST shapes is fragile.
- **0 matches** = pattern wrong. Iterate the pattern; do NOT fall back to per-file `Edit`.
- **Footgun:** `replacement` is a regex template, not a C string. `\n` / `\t` are NOT decoded — they land as literal backslash-n on disk. Put a real newline in the JSON value. Backrefs `$1` / `${name}` ARE interpolated.

Atomic two-phase commit + rollback means a half-applied removal can't leave the tree uncompilable mid-batch — fits the green-always invariant.
