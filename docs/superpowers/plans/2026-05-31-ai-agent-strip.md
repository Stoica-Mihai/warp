# AI Agent Strip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all Warp AI agent subsystems (`ai/blocklist/`, `ai/agent/`, `ai/agent_sdk/`, `ai/ambient_agents/`, and related modules) while keeping the core terminal emulator, MCP, local Drive, and non-AI settings working.

**Architecture:** The AI strip uses the established "removal loop": delete the definition (module/type/method), run `cargo check --features local_fs,gui` to enumerate every break site, fix in batches, verify all 3 gates green, commit. The order is dependency-inverted: strike narrow inner modules first (ambient_agents, agent_sdk), then the agent backend, then the blocklist AI views, then excise AI branches from the core spider files (terminal/view.rs, terminal/input.rs, pane_group/mod.rs, workspace/view.rs). The `crates/ai` engine crate is deleted last when all app callers are gone.

**Tech Stack:** Rust, `cargo check` (oracle), `recast` MCP tool for ≥5-site mechanical rewrites, `python3` for line-range batch deletes on large files.

**Reference docs:** `AGENTS.md` §2–4 (invariants, 3-gate matrix, recast guide), `plan.md` §2 (AI strip status).

**Invariants:**
- `cargo check -p warp` + `cargo check -p warp --tests` + `cargo check -p warp --features local_fs,gui` all 0 errors before every commit.
- Run gates in parallel with separate `CARGO_TARGET_DIR` (see AGENTS.md §4).
- No `#[allow(dead_code)]` cheats — delete the dead code instead.
- When replacing a singleton model with a stub, keep `ctx.add_singleton_model(...)` in `lib.rs`.
- Add a row to `build-size-log.md` after every commit that changes binary size.

---

## Phase Overview

| Phase | Target | Difficulty | Expected LoC |
|---|---|---|---|
| A | Narrow standalone AI modules | Medium | ~5k |
| B | `ai/ambient_agents/` + ambient agent UI | Hard | ~15k |
| C | `ai/agent_sdk/` | Hard | ~20k |
| D | `ai/agent/` backend | Hard | ~22.5k |
| E | `ai/blocklist/agent_view/` + input surface | Very Hard | ~40k |
| F | Remaining blocklist (controller, action_model, etc.) | Very Hard | ~60k |
| G | Excise AI from core spider files | Very Hard | — |
| H | Delete `crates/ai`, remaining deps | Medium | ~25k |

---

## Task 1: Phase A — Narrow standalone AI modules

Strike self-contained AI modules that have no UI rendering in `terminal/view.rs` and whose callers are limited to other `ai/` modules or `lib.rs` init calls.

**Targets in order (each is one commit):**

1. `ai/agent_tips.rs` — used by command palette only; ~673 LoC
2. `ai/active_agent_views_model.rs` + `active_agent_views_model_tests.rs` — tracks open agent views
3. `ai/conversation_details_panel.rs` + `conversation_details_panel_tests.rs` — AI conversation panel
4. `ai/persisted_workspace.rs` — AI workspace state persistence
5. `ai/restored_conversations.rs` — conversation restore-on-launch
6. `ai/agent_conversations_model.rs` + `agent_conversations_model_tests.rs` — conversation data model
7. `ai/agent_events/` — event streaming bridge (driver + message_hydrator)

**Files:**
- Delete: each target module + its test file
- Modify: `app/src/ai/mod.rs` (remove `mod` decl each time)
- Modify: `app/src/lib.rs` (remove singleton registrations + imports)
- Modify: callers surfaced by `cargo check` (fix in batch per commit)

- [ ] **Step 1: Delete `agent_tips.rs` and let cargo check enumerate callers**

```bash
cd ~/Documents/git/warp-upstream
rm app/src/ai/agent_tips.rs
```

Edit `app/src/ai/mod.rs`: remove `pub(crate) mod agent_tips;` and `pub use agent_tips::*;`

- [ ] **Step 2: Run cargo check to find all break sites**

```bash
CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error" | head -40
```

Expected: errors listing every use of `AgentTip`/`agent_tips` types. Fix each by removing the dead import/field/call.

- [ ] **Step 3: Verify 3-gate green and commit**

```bash
CARGO_TARGET_DIR=target/gate-default cargo check -p warp &
CARGO_TARGET_DIR=target/gate-tests   cargo check -p warp --tests &
CARGO_TARGET_DIR=target/gate-feat    cargo check -p warp --features local_fs,gui &
wait
```

Expected: 0 errors on all 3 gates.

```bash
git add -p
git commit -m "refactor: delete ai/agent_tips (−673 LoC)"
```

- [ ] **Step 4: Repeat removal loop for each remaining Phase A target**

For each target in the list above (active_agent_views_model, conversation_details_panel, persisted_workspace, restored_conversations, agent_conversations_model, agent_events):

```
rm -r app/src/ai/<target>   # or rm app/src/ai/<target>.rs
# Edit ai/mod.rs to remove mod decl
# Run: CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error"
# Fix each error (remove dead imports, fields, callers)
# Run 3-gate
# Commit: "refactor: delete ai/<target> (−NNN LoC)"
```

---

## Task 2: Phase B — Ambient agents strip

`ai/ambient_agents/` has 41 external callers. The UI lives in `terminal/view/ambient_agent/` (a separate directory). Delete the model + SDK layer, then delete the UI, then fix the callers.

**Files:**
- Delete: `app/src/ai/ambient_agents/` (9 files: mod.rs, scheduled.rs, spawn.rs, spawn_tests.rs, task.rs, task_tests.rs, telemetry.rs, github_auth_notifier.rs, github_auth_url.rs)
- Delete: `app/src/terminal/view/ambient_agent/` (all files within this directory)
- Modify: `app/src/ai/mod.rs` — remove `pub mod ambient_agents;`
- Modify: `app/src/lib.rs` — remove `ScheduledAgentManager` + `GitHubAuthNotifier` bootstrap + `AgentSource` usages
- Modify: `app/src/pane_group/child_agent.rs` — remove ambient_agents imports; this file may become deletable
- Modify: `app/src/terminal/view.rs` — remove `ambient_agent::*` imports + all ambient agent render branches
- Modify: `app/src/terminal/input.rs` — remove ambient agent input handling
- Modify: `app/src/settings_view/environments_page.rs` + `handoff_environment_creation_modal.rs` + `update_environment_form.rs` — remove ambient agent wiring
- Modify: many others surfaced by `cargo check`

- [ ] **Step 1: Delete ai/ambient_agents/ directory**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/ai/ambient_agents
```

Edit `app/src/ai/mod.rs`: remove `pub mod ambient_agents;`

- [ ] **Step 2: Run cargo check — collect the full error list**

```bash
CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error\[" | sed 's/.*--> //' | sort -u | head -60
```

- [ ] **Step 3: Delete terminal/view/ambient_agent/ UI directory**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/terminal/view/ambient_agent
```

Edit `terminal/view.rs`: remove `mod ambient_agent;` and all uses.

- [ ] **Step 4: Fix remaining callers (pane_group/child_agent.rs, lib.rs, settings_view, persistence)**

Work through errors from `cargo check` output. Key patterns:
- `AgentSource` → remove the field/param/arm (it fed ambient agent spawning)
- `ScheduledAgentManager` → remove from lib.rs singleton registration
- `GitHubAuthNotifier` → remove from lib.rs bootstrap
- `IsSharedSessionCreator` → this type also gates ambient agent child panes; stubs may be needed

- [ ] **Step 5: 3-gate verify + commit**

```bash
CARGO_TARGET_DIR=target/gate-default cargo check -p warp &
CARGO_TARGET_DIR=target/gate-tests   cargo check -p warp --tests &
CARGO_TARGET_DIR=target/gate-feat    cargo check -p warp --features local_fs,gui &
wait
git commit -m "refactor: delete ai/ambient_agents + terminal/view/ambient_agent/ (−NNN LoC)"
```

---

## Task 3: Phase C — Agent SDK strip

`ai/agent_sdk/` (~40+ files) contains the harness drivers for claude-code, codex, etc. It's a client-side SDK for running AI agents. Dependencies: `agent_sdk` → `agent` (uses agent types). External callers: primarily `lib.rs` and `pane_group/`.

**Files:**
- Delete: `app/src/ai/agent_sdk/` entire directory
- Modify: `app/src/ai/mod.rs` — remove `#[cfg(not(target_family = "wasm"))] pub mod agent_sdk;`
- Modify: `app/src/lib.rs` — remove `ConnectedSelfHostedWorkersModel` + `HarnessAvailabilityModel` + other agent_sdk bootstrap
- Modify: `app/src/pane_group/pane/local_harness_launch.rs` + `local_harness_launch_tests.rs` — likely deletable (harness launch pane)
- Modify: `app/src/terminal/view/docker_sandbox/mod.rs` — remove agent_sdk usages
- Modify: callers surfaced by cargo check

- [ ] **Step 1: Delete ai/agent_sdk/**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/ai/agent_sdk
```

Edit `app/src/ai/mod.rs`: remove the `#[cfg(not(target_family = "wasm"))] pub mod agent_sdk;` line.

- [ ] **Step 2: Run cargo check — collect errors**

```bash
CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error" | head -60
```

- [ ] **Step 3: Fix callers**

Key patterns to watch for:
- `HarnessAvailabilityModel` — remove from lib.rs singleton + callers
- `ConnectedSelfHostedWorkersModel` — remove from lib.rs
- `LocalHarnessLaunchPane` / `LocalHarnessPaneModel` — `pane_group/pane/local_harness_launch.rs` may become deletable
- `IPaneType::LocalHarness` variant + `LeafContents::LocalHarness` — remove from enum cascade
- Cloud provider usages in settings_view environments_page

- [ ] **Step 4: 3-gate verify + commit**

```bash
git commit -m "refactor: delete ai/agent_sdk/ (−NNN LoC)"
```

---

## Task 4: Phase D — Agent backend strip

`ai/agent/` (22.5k LoC) is the core agent driver — conversation data, linearization, todos, actions. It's the "brain" behind Agent Mode. External callers include `terminal/view.rs` (heavy), `blocklist/` (heavy), `lib.rs`.

**Files:**
- Delete: `app/src/ai/agent/` entire directory (all subdirs)
- Modify: `app/src/ai/mod.rs` — remove `pub(crate) mod agent;`
- Modify: `app/src/lib.rs` — remove `AIAgentTodoList`, `FileEdit`, `TodoOperation` pub uses + `AgentConversationsModel` + `AgentNotificationsModel` bootstrap
- Modify: `app/src/terminal/view.rs` — remove all `agent::*` imports + handling (~25 import lines)
- Modify: `app/src/ai/blocklist/` — remove agent imports (blocklist controller imports agent heavily)
- Modify: `app/src/ai/agent_management/` — delete if it feeds only into agent (verify first)
- Modify: many others

**Strategy note:** The `agent/` module is imported by both `blocklist/` and `terminal/view.rs`. If blocklist isn't deleted yet (Phase F is after), we'll need to stub the agent types that blocklist uses, or do Phase D+F together in the same session with no intermediate commit.

Option A (stub then delete): Replace `ai/agent/` with a stub that exports the types blocklist needs, then delete the stub in Phase F when blocklist goes.
Option B (combined): Delete agent + blocklist in a single session without intermediate commit (riskier but cleaner).

**Recommendation:** Option A — stub first:

- [ ] **Step 1: Identify which agent types blocklist still needs**

```bash
grep -r "crate::ai::agent::\|ai::agent::" ~/Documents/git/warp-upstream/app/src/ai/blocklist/ | grep -v "_tests\|//|test" | grep "use " | sed 's/.*:://' | sort -u | head -30
```

- [ ] **Step 2: Delete ai/agent/ — let cargo check list blocklist's needs**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/ai/agent
```

Edit `app/src/ai/mod.rs`: remove `pub(crate) mod agent;`

Run: `CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error" | head -80`

- [ ] **Step 3: Fix all callers except within blocklist/**

Fix `terminal/view.rs`, `lib.rs`, `pane_group/`, etc. For blocklist callers: if they're too numerous, create a thin `app/src/ai/agent.rs` stub that re-exports only what blocklist needs (empty structs/enums with `#[derive(Default)]`).

- [ ] **Step 4: 3-gate verify + commit**

```bash
git commit -m "refactor: delete ai/agent/ (−NNN LoC)"
```

---

## Task 5: Phase E — Blocklist agent_view + AI input surface

`ai/blocklist/agent_view/` is the UI for the agent mode conversation pane. `ai/blocklist/agent_view/agent_input_footer/` is the AI input bar. These feed into `terminal/view.rs` which has the agent view render paths.

**Files:**
- Delete: `app/src/ai/blocklist/agent_view/` entire directory
- Delete: `app/src/terminal/view/agent_view.rs` (terminal-side agent view)
- Delete: `app/src/terminal/view/load_ai_conversation.rs`
- Delete: `app/src/terminal/view/pending_user_query.rs`
- Delete: `app/src/terminal/view/use_agent_footer/`
- Modify: `app/src/ai/blocklist/mod.rs` — remove `pub mod agent_view;`
- Modify: `app/src/terminal/view.rs` — remove agent view render branches (heavy — this file is 203 AI refs)
- Modify: `app/src/terminal/view/pane_impl.rs` — remove agent pane methods
- Modify: `app/src/terminal/input.rs` — remove AI input mode branches

**Strategy:** Use the sink-first approach. Delete agent_view/ → cargo check shows exactly what terminal/view.rs used from it → remove those method calls/fields from terminal/view.rs → iterate until green.

This is the most labor-intensive phase. Budget 2-3 sessions.

- [ ] **Step 1: Delete ai/blocklist/agent_view/**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/ai/blocklist/agent_view
```

Edit `app/src/ai/blocklist/mod.rs`: remove `pub mod agent_view;` line.

- [ ] **Step 2: Collect errors from cargo check**

```bash
CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error" | wc -l
CARGO_TARGET_DIR=target/gate-feat cargo check -p warp --features local_fs,gui 2>&1 | grep "^error" | head -40
```

- [ ] **Step 3: Delete terminal/view AI surface files**

```bash
rm ~/Documents/git/warp-upstream/app/src/terminal/view/agent_view.rs
rm ~/Documents/git/warp-upstream/app/src/terminal/view/load_ai_conversation.rs
rm ~/Documents/git/warp-upstream/app/src/terminal/view/pending_user_query.rs
rm -r ~/Documents/git/warp-upstream/app/src/terminal/view/use_agent_footer
```

Edit `terminal/view.rs`: remove the `mod` declarations for these files.

- [ ] **Step 4: Excise AI branches from terminal/view.rs**

For each error in terminal/view.rs, remove the AI-specific import/field/method/render branch. The non-AI terminal render path must be preserved. Key things to keep:
- Normal block rendering
- PTY/command execution
- Input handling for classic (non-agent) mode
- Context chips

Key things to remove:
- `ask_blocklist_ai` method
- `agent_view` field + `build_agent_view` + `render_agent_view`
- `ai_block` related render calls
- `AskAIType` usage

- [ ] **Step 5: 3-gate verify + commit (or intermediate WIP commits)**

For large file changes, commit after each logical chunk that passes cargo check, even if not a full phase. Use WIP commit style: `wip: excise agent_view from terminal/view (partial)`.

Final clean commit: `refactor: delete ai/blocklist/agent_view + terminal AI views (−NNN LoC)`

---

## Task 6: Phase F — Remaining blocklist modules

After agent_view is gone, the remaining blocklist modules (controller, action_model, history_model, input_model, orchestration, etc.) have no UI callers in terminal/view.rs. Delete them.

**Files:**
- Delete: `app/src/ai/blocklist/action_model/` (action execution: shell, file edits, MCP calls)
- Delete: `app/src/ai/blocklist/controller/` (BlocklistAIController)
- Delete: `app/src/ai/blocklist/history_model.rs` (BlocklistAIHistoryModel, AIQueryHistory)
- Delete: `app/src/ai/blocklist/input_model.rs` (BlocklistAIInputModel)
- Delete: `app/src/ai/blocklist/context_model.rs`
- Delete: `app/src/ai/blocklist/orchestration_events.rs` + `orchestration_topology.rs` + `orchestration_event_streamer.rs`
- Delete: `app/src/ai/blocklist/task_status_sync_model.rs`
- Delete: `app/src/ai/blocklist/passive_suggestions.rs`
- Modify: `app/src/lib.rs` — remove `BlocklistAIHistoryModel`, `BlocklistAIPermissions`, `OrchestrationPillBarModel`, `OrchestrationEventService`, `TaskStatusSyncModel` singleton registrations
- Modify: `app/src/ai/blocklist/mod.rs` — remove `mod` decls for deleted modules + their re-exports

**Strategy:** Same removal loop. The key cleanup here is `lib.rs` bootstrap (the singleton models that must be removed to avoid runtime panics).

- [ ] **Step 1: Remove singleton registrations from lib.rs first**

These will panic at runtime if left registered without the type:
```rust
// Remove from lib.rs:
ctx.add_singleton_model(ai::blocklist::orchestration_events::OrchestrationEventService::new);
ctx.add_singleton_model(ai::blocklist::task_status_sync_model::TaskStatusSyncModel::new);
// and the OrchestrationPillBarModel::new call
```

Also remove `ai::blocklist::agent_view::editor::init(ctx);` and other init calls.

- [ ] **Step 2: Delete remaining blocklist modules one cluster at a time**

Per-cluster removal loop (each cluster is one commit if green):
1. `orchestration_events.rs` + `orchestration_topology.rs` + `orchestration_event_streamer.rs` + `task_status_sync_model.rs`
2. `history_model.rs` + `input_model.rs` + `context_model.rs`
3. `action_model/` directory
4. `controller/` directory
5. `passive_suggestions.rs` + remaining leaf files

- [ ] **Step 3: 3-gate verify + commit each cluster**

```bash
git commit -m "refactor: delete ai/blocklist/<cluster> (−NNN LoC)"
```

---

## Task 7: Phase G — Excise AI from core spider files

After the blocklist and agent systems are deleted, terminal/view.rs and its siblings still have AI plumbing (imports for deleted types, dead match arms, dead fields). Clean them up.

**Files:**
- Modify: `app/src/terminal/view.rs` — remove remaining AI dead code (fields, imports, dead branches)
- Modify: `app/src/terminal/input.rs` — remove AI input dead code
- Modify: `app/src/pane_group/mod.rs` — remove AI pane types (AIDocumentPane, AiFact pane, etc.)
- Modify: `app/src/pane_group/pane/mod.rs` — remove AI pane variant arms
- Modify: `app/src/pane_group/pane/ai_document_pane.rs` + `ai_fact_pane.rs` — delete these files
- Modify: `app/src/workspace/view.rs` — remove AI conversation list, free tier limit modal, bonus grant
- Modify: `app/src/workspace/view/conversation_list/` — delete these files
- Modify: `app/src/workspace/view/free_tier_limit_hit_modal.rs` + `app/src/workspace/bonus_grant_notification_model.rs` — delete

- [ ] **Step 1: Delete leaf AI view files in pane_group/**

```bash
rm ~/Documents/git/warp-upstream/app/src/pane_group/pane/ai_document_pane.rs
rm ~/Documents/git/warp-upstream/app/src/pane_group/pane/ai_fact_pane.rs
```

Remove their `mod` decls from `pane_group/pane/mod.rs`. Let cargo check enumerate remaining pane_group AI references.

- [ ] **Step 2: Remove IPaneType/LeafContents AI variants**

`LeafContents::AIDocument`, `LeafContents::AIFact`, `LeafContents::CodeDiff` (if AI-only) etc. need cascade removal through pane_group/mod.rs match arms.

- [ ] **Step 3: Delete workspace AI view files**

```bash
rm -r ~/Documents/git/warp-upstream/app/src/workspace/view/conversation_list
rm ~/Documents/git/warp-upstream/app/src/workspace/view/free_tier_limit_hit_modal.rs
rm ~/Documents/git/warp-upstream/app/src/workspace/bonus_grant_notification_model.rs
```

Fix workspace/view.rs and workspace/mod.rs accordingly.

- [ ] **Step 4: Clean remaining dead imports/fields in core spider files**

Use `cargo check` warnings (not just errors) to find dead fields/imports. Work through each file:
- `terminal/view.rs`: remove dead `ai_*` fields, dead event handler arms
- `terminal/input.rs`: remove AI input mode logic
- `workspace/view.rs`: remove AI conversation UI

- [ ] **Step 5: 3-gate verify + commit**

```bash
git commit -m "refactor: excise AI branches from core spider files (−NNN LoC)"
```

---

## Task 8: Phase H — Delete remaining ai/ modules and crates/ai

Once all app callers of `crates/ai` are removed, delete the engine crate.

**Files:**
- Delete: `crates/ai/` entire directory (~75 files, 25k LoC)
- Delete: `app/src/ai/` remaining modules (mcp is KEPT; others like `llms`, `skills`, `facts`, `predict`, `voice`, `outline`, `cloud_environments`, `document`, `harness_display`, `harness_availability`, etc. delete if zero live callers)
- Modify: `Cargo.toml` — remove `crates/ai` workspace member + `app` dependency on it
- Modify: `app/src/lib.rs` — remove remaining ai-engine imports

**Check before deleting each module:**
```bash
grep -rn "crate::ai::<module>\|ai::<module>" ~/Documents/git/warp-upstream/app/src --include="*.rs" | grep -v "_tests\|//" | wc -l
```
Zero = safe to delete.

**Note on MCP:** `app/src/ai/mcp/` is KEPT. MCP is a non-AI protocol that the user explicitly wants to preserve (it's the Model Context Protocol, not Warp's AI). Verify with user before touching any file in `ai/mcp/`.

- [ ] **Step 1: Audit remaining ai/ submodules for live callers**

```bash
for mod in llms skills facts predict voice outline cloud_environments document harness_display harness_availability cloud_agent_config cloud_agent_settings connected_self_hosted_workers codebase_auto_indexing get_relevant_files generate_block_title generate_code_review_content loading; do
  count=$(grep -rn "${mod}" ~/Documents/git/warp-upstream/app/src --include="*.rs" | grep -v "app/src/ai/${mod}" | grep -v "_tests\|//" | wc -l)
  echo "${mod}: ${count} external refs"
done
```

- [ ] **Step 2: Delete each zero-caller ai/ module**

For each with 0 external refs:
```bash
rm -r ~/Documents/git/warp-upstream/app/src/ai/<module>
# Edit ai/mod.rs to remove mod decl
# Run 3-gate verify
# Commit: "refactor: delete ai/<module> (−NNN LoC)"
```

- [ ] **Step 3: Delete crates/ai once app has zero ai-crate imports**

```bash
# Verify zero direct ai-crate imports (::ai::)
grep -rn "^use ::ai::\|^use ai::" ~/Documents/git/warp-upstream/app/src --include="*.rs" | wc -l
# Should be 0
rm -r ~/Documents/git/warp-upstream/crates/ai
```

Edit workspace `Cargo.toml`: remove `crates/ai` from `[workspace] members`. Edit `app/Cargo.toml`: remove `ai` dep.

- [ ] **Step 4: Final 3-gate verify + build + size log**

```bash
CARGO_TARGET_DIR=target/gate-default cargo check -p warp &
CARGO_TARGET_DIR=target/gate-tests   cargo check -p warp --tests &
CARGO_TARGET_DIR=target/gate-feat    cargo check -p warp --features local_fs,gui &
wait

# If all green:
cargo build --bin sublight --features gui
stat -c%s target/debug/sublight
```

Update `build-size-log.md` with the final binary size.

```bash
git commit -m "refactor: delete crates/ai (−NNN LoC, −N MB)"
```

---

## Verification after all phases

```bash
# No remaining agent-mode references in non-AI code
grep -rn "agent_mode\|AgentMode\|agent_view\|AgentView\|ask_blocklist_ai\|BlocklistAI" \
  ~/Documents/git/warp-upstream/app/src --include="*.rs" \
  | grep -v "app/src/ai/" | grep -v "_tests" | head -20
# Expected: empty or only innocuous string literals

# App launches
cargo run --bin sublight --features gui
# Expected: terminal opens immediately, no crash
```

---

## Key lessons from prior strips

1. **Singleton models must stay registered.** If any code path calls `SomeModel::as_ref(ctx)`, the model must be in lib.rs even as a stub. Remove the registration only when the last caller is gone.

2. **Sink-first collapse beats bottom-up.** Delete the UI view (sink) first — cargo check then enumerates every feeder. This avoids stub proliferation.

3. **Never trust incremental cargo check alone.** After large multi-file sweeps, run one gate cold or use `cargo run` to bust the incremental cache before trusting exit 0.

4. **recast for ≥5-site mechanical changes.** Single `recast_preview` + `recast_apply` beats per-file Edit loops for renames/drops across many files.

5. **MCP is NOT AI.** `app/src/ai/mcp/` implements the Model Context Protocol (a generic tool-calling interface). Keep it. Only delete if user explicitly says to.

6. **`session_sharing_protocol` crate — KEEP.** `SharedSessionSource` + `SessionSourceType` are load-bearing for ambient agent pane creation. Do not delete.

---

## Session 2026-05-31b progress (Group B relocations complete; Group A sweep in progress)

Started from ~143 errors (prior pass deleted `ai/blocklist/agent_view/` + the AI view files). Drove down via:

### Group B relocations — DONE (genuinely-used types moved to surviving modules)

1. **`AgentToolbarItemKind`** (+ `ToolbarAvailability`) → **`app/src/context_chips/toolbar.rs`** (new file, `pub mod toolbar;` in `context_chips/mod.rs`). Dropped `defaults_for_mode` (only consumer of deleted `AgentToolbarEditorMode`; surviving callers use `default_left`/`cli_default_*` directly). `is_available_during_handoff_compose` changed `pub(super)`→`pub(crate)`. Callers updated: `terminal/session_settings.rs`, `chip_configurator/mod.rs`, `terminal/view.rs` (import only).

2. **`AgentViewState` + `AgentViewEntryOrigin` + `AgentViewDisplayMode` + `ENTER_OR_EXIT_CONFIRMATION_WINDOW` + `render_block_container` + `agent_view_bg_fill` + `agent_view_bg_color` + `get_agent_view_entry_block_position_id`** → **`app/src/terminal/view/agent_view_state.rs`** (new file, `pub mod agent_view_state;` in `terminal/view.rs`). Dropped dead methods `should_autotrigger_request`/`AutoTriggerBehavior` (test-only) and `was_conversation_modified_since_opening` (only `BlocklistAIHistoryModel` dep, no live caller). `AgentViewEntryOrigin` kept ALL variants (cheap enum). Import-path callers updated: `terminal/model/block.rs`, `model/blocks.rs`, `block_list_element.rs`, `block_list_viewport.rs`, `pane_impl.rs`, `rich_content.rs`, `ai/ai_document_view.rs`, `ambient_agent/block/entry.rs`, `integration_testing/{agent_mode/mod,terminal/assertion}.rs`, + model test files.

3. **`AgentInputButtonTheme`** → **`app/src/terminal/view/ambient_agent/button_theme.rs`** (new file, `mod button_theme;`). Callers: ambient `model_selector.rs`, `harness_selector.rs` (now `use super::button_theme::AgentInputButtonTheme;`).

### Group A — guard collapses DONE
- `is_cloud_mode_input_v2_composing(<arg>)` → `false` at all 31 call sites (input.rs ×16, slash_commands/mod.rs ×13, view_tests.rs ×2) + view.rs chained call (1).
- `should_show_auth_secret_ftux(ctx)` guard → `if false` (×2 in input.rs). `auth_secret_ftux_view()` infra KEPT (only the predicate was deleted).
- `is_cloud_agent_pre_first_exchange`: dropped `agent_view_controller` param + dead `AgentViewState::Active`/`origin.is_cloud_agent()` gate (now gates purely on `view_model.is_local_to_cloud_handoff()`). 5 call sites de-argued: view.rs ×2, status_bar.rs, block/view_impl.rs, pane_impl.rs. Import dropped from `ambient_agent/mod.rs`.
- `CloudModeSetupTextBlock::new`: dropped `agent_view_controller` param + dead history-subscription block. Caller `ambient_agent/view_impl.rs` de-argued.

### Group A — REMAINING (the big interdependent deletion sweep; ~100 errors left)
The dominant remaining chains are struct-field removals from two huge structs:

- **`TerminalView` (terminal/view.rs, ~59 errors)** — remove fields + all uses: `agent_view_controller` (32 refs: field, ctor `AgentViewController::new`, accessor `agent_view_controller()`, subscription, ~20 `.as_ref(ctx).is_active()/.is_fullscreen()` guards → collapse to false), `ephemeral_message_model` (6), `orchestration_pill_bar` (4, `OrchestrationPillBar::new`). Plus deleted-type uses: `AgentViewControllerEvent` (4, event match arms), `InlineAgentViewHeader` (4), `AgentViewZeroStateBlock`/`AgentViewZeroStateEvent` (5), `AgentViewHeaderTheme`/`AgentViewHeaderDisabledTheme` (4), `AgentViewEntryBlockParams` (2), `fork_from_last_known_good_state_exchange_id` (3). Then fix the import block at view.rs lines ~211-219 (keep only relocated names, repoint to `crate::terminal::view::agent_view_state` / `context_chips::toolbar`).
  - NOTE: view.rs line ~18363 uses `agent_view::ENTER_AGAIN_TO_SEND_MESSAGE_ID` — remove use.
  - NOTE: 2 leftover `if false {` from the guard collapse (view.rs ~one site, input.rs ~two) — fine to leave or simplify after green.

- **`Input` (terminal/input.rs, ~23 errors)** — already removed: `inline_conversation_menu_view` field + ctor, `agent_input_footer` ctor + subscription block (lines ~2089-2101, 2283-2383). STILL TODO: `agent_input_footer` field (line ~1503 area) + accessor `agent_input_footer()` + ~12 body uses (focus/`has_open_chip_menu`/`.update()` — trace each; the footer focus/chip-menu logic routes live input, decide keep-vs-drop per call), `agent_shortcut_view_model` field + `AgentShortcutViewModel::new`, `ephemeral_message_model` field+param+ctor, `agent_view_controller` field+param+ctor+subscription + ~10 `.as_ref(ctx).is_*` guards, `sort_environments_by_recency` (1 use). Then import block lines 145-150 + conversations import line 262-264 (`InlineConversationMenuEvent/View` — remove; module deleted). E0560: `DataSourceArgs` has no field `agent_view_controller` at input.rs ~3063/3075 — drop those struct-init lines. E0061 menu `::new` calls (input.rs ~2662, 3108-3205) — drop the `agent_view_controller.clone()`/`&agent_view_controller` argument (constructors already de-threaded).

- **Dead ai/blocklist satellites (each 1-3 errors)**: `block/status_bar.rs` (builds `AgentMessageBar`/`ChildAgentStatusCard`, holds `AgentViewController`+`EphemeralMessageModel` — heavily entangled; likely whole status-bar-for-agent-view path is dead), `history_model.rs`, `context_model.rs` (`AgentViewControllerEvent` at line 261, `EnterAgentViewError`), `block/view_impl/orchestration.rs` (`OrchestrationAvatar`/`OrchestrationConversationLinks` imports), `block.rs`, `input_model.rs` (`AgentViewEntryOrigin::ClearBuffer`), `controller.rs`, `controller/slash_command.rs`, `block/cli_controller.rs`, `inline_action/run_agents_card_view.rs` (`render_static_agent_pill`), `usage/conversation_usage_view.rs` (orchestration pill imports), `handoff/touched_repos.rs` (`sort_environments_by_recency`). For ai/blocklist/* imports that point at `agent_view::AgentViewEntryOrigin`, repoint to `crate::terminal::view::agent_view_state::AgentViewEntryOrigin`; for genuinely-deleted types (`AgentViewController`, orchestration views, `EnterAgentViewError`, `render_static_agent_pill`), remove the use.

- **Menu view files (1 error each)**: `inline_menu/view.rs`, `inline_history/view.rs`, `cloud_mode_v2_history_menu.rs`, `inline_history/data_source.rs`, `slash_commands/{view,search_item}.rs`, `slash_command_model.rs`, `{skills,rewind,repos,prompts,plans,models,user_query}/view.rs`, `message_bar/{common,attached_context}.rs`, `inline_menu/positioning.rs` — mostly `AgentViewController` import + 1 use, or `render_keystroke_with_color_overrides`/`AgentShortcutViewModel` (deleted `shortcuts` module). `message_bar/common.rs` `agent_view_bg_color` → repoint to `terminal::view::agent_view_state`.

- **lib.rs (3 errors)**, **pane_group/pane/{mod,terminal_pane}.rs**, **workspace/view.rs** (`AgentToolbarEditorMode`/`AgentToolbarEditorModal`/`AgentToolbarEditorEvent` at lines 150-151 — deleted editor module), **workspace/view/conversation_list/{view,item}.rs**, **auth/mod.rs** (`OrchestrationPillBarModel` import), **terminal/view/context_menu.rs** (`fork_ai_conversation` references deleted types), **terminal/view/zero_state_block.rs** (`AgentViewController`/`AgentViewControllerEvent` + 2 deleted keystroke consts `ENTER_AGENT_VIEW_NEW_CONVERSATION_KEYSTROKE`/`ENTER_CLOUD_AGENT_VIEW_NEW_CONVERSATION_KEYSTROKE`), **mock_terminal_manager.rs**.

### Method to finish
Remove `agent_view_controller`/`ephemeral_message_model`/`orchestration_pill_bar` fields from `TerminalView` and `Input` first (collapse `.is_active()`/`.is_fullscreen()` guards to `false`), then the cargo errors enumerate the rest. Re-run 3-gate cold before commit. Tree is currently RED (does not compile) — do NOT commit until green.
