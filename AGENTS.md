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

telemetry · wasm-orphan crates · onboarding crate + flags · Sentry crash-reporting · Sublight rebrand · channel enum cascade · autoupdate pipeline · session-sharing (all vestiges incl. `SharedSessionStatus` + `is_shared_session_viewer` cascade) · Warp Drive server-sync + SyncQueue + ObjectClient · SharingDialog + permission-CRUD · login gate bypass · auth gate UI · AuthManager login stub · Firebase crate · server-driven A/B experiments · dead AuthClient privacy-sync · AuthView/AuthOverrideWarningModal cloud-gate UI · billing/teams/platform/referrals pages · AI assistant panel + Warp AI command search · AI settings pages + execution profile editor · execution profiles data model + inline profile selector · ScheduledAgentManager registration + AgentSource computation · **Phase C: `ai/agent_sdk/` STRIP COMPLETE** (`d3eb301d` + `889d83dd`) · **Phase E+F complete: `ai/blocklist/agent_view/` deleted + AI view files** (`94c6be3f`) · `TextLocation` relocated to `util/text_location` + `LinkActionConstructors` to `util/link_detection` (`73dd201c`) · **Phase G-preview COMPLETE: `ai_controller` removed from TerminalView struct** (`1f600e66`, −10.97 MB)

**Current state:** Binary **794.3 MB** (−49.9 MB total vs 844.2 MB baseline). **0 errors / ~400 warnings** (all AI territory, deferred, no `#[allow(dead_code)]` cheats). App always starts in Terminal.

**Phase F steps 1–3 DONE** (2026-05-31):
- **Step 1** (`202b79e0`, −7.96 MB): `controller/` + `passive_suggestions/` deleted. Types relocated: `SessionContext`, `RequestInput`, `ResponseStreamId`, `ClientIdentifiers` → `ai/blocklist/*.rs`. −6,081 LoC.
- **Step 2** (`c42b65cb`, 0 MB linker-invisible): `action_model/` deleted. All types → `action_stubs.rs`. −13,421 LoC.
- **Step 3** (`0576df88`, 0 MB linker-invisible): orchestration cluster deleted (orchestration_events, orchestration_event_streamer, orchestration_topology, orchestration_conversation_links, task_status_sync_model, local_shared_session_link_model). Types → `orchestration_stubs.rs`. −6,369 LoC.

**Next: Phase F steps 4–7** — remaining model layer deletions:

4. **Delete `ai/blocklist/context_model.rs`** — 10 external files; moderate scope
5. **Delete `ai/blocklist/input_model.rs`** — 33 external files; ai_input_model still in Input struct  
6. **Delete `ai/blocklist/history_model.rs` + `history_model/`** — 62 external files; ~40 stub methods needed (largest remaining single target)
7. **Delete `ai/blocklist/permissions.rs`, `persistence.rs`** — scope TBD
8. **Delete `ai/blocklist/block.rs` + `block/`** — requires relocating first:
   - `secret_redaction` → `app/src/secret_redaction.rs`
   - `keyboard_navigable_buttons`, `toggleable_items`, `numbered_button`, `compact_agent_input`, `inline_action_header`, `inline_action_icons`, `requested_action`, `WithContentItemSpacing` → `terminal/view/` (used by `init_project/`, `init_environment/`, `ssh_remote_server_choice_view`, `ambient_agent/`)
9. **lib.rs:** remove `BlocklistAIHistoryModel`, `BlocklistAIPermissions` registrations (OrchestrationEventService/TaskStatusSyncModel/OrchestrationEventStreamer/LocalSharedSessionLinkModel already updated to use stubs)

**STUB PATTERN (established this session):**
- Delete the module files, create `ai/blocklist/<name>_stubs.rs`, stub all types with no-op implementations, re-export from `mod.rs`.
- For types still needed by non-deleted ai/agent/ code: relocate to new `ai/blocklist/<type>.rs` file instead.
- Run cargo check oracle loop to discover missing stub methods — add them iteratively.
- Stubs are linker-invisible (binary doesn't shrink) until the CALLERS in block/, inline_action/ are deleted.

**Phase F prerequisite note:** `ai_action_model`/`ai_input_model`/`ai_context_model` are still in Input struct AND TerminalView struct. Removing them from Input requires fixing 50+ method bodies. The stubs approach means these fields now reference no-op implementations — fine to remove in a later pass alongside block/ deletion.

**Phase F lessons (2026-05-31):**
- **Stub pattern**: Delete module, create `<name>_stubs.rs` in `ai/blocklist/`, stub all types (unit structs, empty enums, no-op methods), re-export from `mod.rs`. Cargo check oracle finds missing methods iteratively.
- **Relocate pattern** (for types still used by non-deleted ai/agent/ code): create minimal file in `ai/blocklist/` with just the struct/type, re-export from mod.rs. Consumers keep same import path.
- **Linker visibility**: Stubs keep binary size flat. Real shrink comes when the CALLERS (block/, inline_action/) are deleted — deferred to step 8.
- **passive_suggestions was dead** — all handler methods already deleted in G-preview; types were dead imports. Safe to delete with controller.
- **orchestration_conversation_links functions**: UI rendering functions (conversation_navigation_card_with_icon returning `Box<dyn Element>`) need a warpui stub: `warpui::elements::Empty::new().finish()`.
- **action_stubs.rs** grew to 400+ lines across 3 iterations. The cargo check oracle reliably finds missing variants/methods.
- stub type files need `#[derive(Clone)]` for types used with `.clone()` calls.
- **SingletonEntity impls**: All stub models need `impl SingletonEntity for X {}` so callers can use `X::handle(ctx)` and `X::as_ref(ctx)` patterns.
- **run_agents_to_start_agent_mode** and **coerce_integer_args** are non-trivial functions used by block/ internals — copied verbatim from deleted action_model into action_stubs.rs (they only depend on crate/workspace deps, no deleted types).

**Phase G-preview lessons (2026-05-31):**
- `ai_render_context` + `cli_subagent_views` woven into `block_list_element.rs` rendering — keep as static-default fields, not removed from struct.
- **Local-only variable pattern**: create `ai_controller` + `cli_subagent_controller` in `new()` as locals, pass to Input — Input works unchanged.
- Stub getters returning `&ModelHandle` let external callers chain `.as_ref(ctx)`.
- `#[ignore]` suppresses test *execution* but not *compilation* — must empty the test body.
- `Input::new()` keeps all 18 params including `ai_controller` + `cli_subagent_controller` — removing from Input is Phase F work.
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
