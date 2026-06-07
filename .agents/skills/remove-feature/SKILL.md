---
name: remove-feature
description: Surgically remove a whole product feature (especially a cloud/backend-coupled one) from a large Rust codebase while keeping the tree green at every commit. Use when stripping a feature/subsystem (teams, billing, cloud-handoff, voice, a settings page, an RPC) — not for removing a single feature flag (use remove-feature-flag for that). Encodes the classification framework, the traps that bite ("names lie", "emitted ≠ alive", schema-bound deser, runtime-dead-but-compiled), the gate matrix, and the increment ordering learned doing the Warp cloud strip.
---

# remove-feature

Surgically remove a whole feature/subsystem from a large Rust codebase **without ever going red**. Distilled from the Warp→Sublight cloud strip (telemetry, AI agent, Warp Drive, Bedrock, warpify, cloud-handoff, teams/billing).

## When to use
- Removing a feature, subsystem, RPC, settings page, or backend-coupled capability.
- The cut spans many files and/or touches a core model, the render path, or a daemon/transport.
- NOT for a single feature-flag retirement → use `remove-feature-flag`.

## The prime directive
**The tree compiles with 0 errors after every commit.** Never delete a definition before its callers are refactored off it. Remove feature-by-feature, commit at green checkpoints, in small reviewable increments. A half-removed feature that doesn't compile is worse than not starting.

## The core loop
1. Delete (or neuter) the definition — enum variant, module, method, trait, struct, action.
2. Run `cargo check` and let the compiler **enumerate every break site**.
3. Fix the breaks in batches (remove call sites, collapse dead branches).
4. Repeat to 0 errors. Then 3-gate, then commit.

The compiler is the work-list. Don't try to find all callers by hand first — delete the keystone, read the errors.

## STEP 0 — verify before you start (do not trust docs)
Plan files / tracker docs / prior memories accumulate **stale "TODO" entries for work already done**, and **inherited claims that are wrong**. Three times in one session a "remaining target" turned out already-deleted.
- Before working any "remaining"/"NEXT" target: `ls` the dir / pickaxe the symbol (`git log -S Symbol`) / grep for live refs. Confirm it still exists and is still live.
- Treat every "KEEP because X" note as a hypothesis to re-verify, not fact. (One KEEP rationale this session — "TeamsChanged drives PrivacySettings" — was simply false; the real subscribers were different. The KEEP held, the reason didn't.)

## The classification framework (the heart of it)
For **every** symbol the feature touches, classify it. Most mistakes are misclassification, not bad edits.

### DELETE — feature-specific and dead
Warp/cloud/feature-proprietary code with no live consumer. Verify via the 3-gate intersection (below), then delete.

### COLLAPSE — runtime-dead but compiled
The code still compiles and is *called*, but the predicate is permanently false / the accessor permanently empty because the backend that fed it is gone.
- e.g. `is_cloud_handoff_enabled()` → always `false`; `current_team()` → always `None`; `has_teams()` → always `false`.
- These are NOT simple deletes: live callers depend on the method existing. Collapse the **dead branch at each call site** to the always-taken path, then the method may become 0-caller and deletable.
- Behavior-preserving collapse needs no runtime test **if you prove the output is identical** (compute it from the defaults — e.g. SSH-wrapper collapse: all branches yielded `true` on defaults, and the setting that could change it was already UI-deleted).

### KEEP — shared / dual-use infrastructure
The thing is *named* for the feature but actually serves something kept. **Names lie.** Verified examples:
- `TeamClient` / `workspaces_metadata()` — the *workspace* fetcher (sole method), not team CRUD.
- `TeamUpdateManager` — the *workspace-metadata polling* loop, not team management.
- `codebase_indices` sqlite table + `Upsert/DeleteCodebaseIndexMetadata` events — persist generic *LSP WorkspaceMetadata*, not the embedding index.
- A shared fetch payload (`WorkspacesMetadataWithPricing`) carries BOTH the kept workspace list AND the removed teams — so the transport stays even though the team data goes.
**Rule:** before deleting anything named after the feature, open it and read what it *does* + who calls it. Trace the actual data, not the name.

### KEEP — schema-bound wire/DB types
cynic GraphQL enums/structs and proto messages mirror the live server schema. Deleting a variant breaks the schema match / deserialization even if it never fires.
- Keep them; map unknown→`Unknown` or deserialize-into-nothing.
- Examples kept: `JsonObjectType::Preference`/`JsonPreference`, Bedrock `ManagedSecretValue` variants, `LLMModelHost` wire variant, proto `UploadHandoffSnapshot{,Response}`, `GqlDiscoverableTeamData`.
- **Trap:** a response struct may need a field/type purely to *deserialize* the server payload (e.g. the workspaces_metadata response carried a `teams` array → `Workspace.teams` + app `Team` were load-bearing for deser even when always empty). Removing them = a gql_convert reshape, not a delete. Check `crates/graphql` before deleting an app type that mirrors a response.

## The traps (each cost real time)
- **"Emitted ≠ alive."** An event variant that's still `ctx.emit(...)`-ed but has **zero subscribers** is dead scaffolding, not live code. "Still emitted" is NOT a keep-rationale. Trace every kept item to a live **consumer/subscriber/reader**, not just a live emitter. (Left the joinable-teams discovery loop in once on this exact mistake — poll→store→emit-into-the-void.)
- **Feature-gated callers hide from the default gate.** `#[cfg(feature = "local_fs")]` code isn't compiled by `cargo check -p warp` (no features). A symbol that looks dead in the default gate may have a real caller under a feature. Always check all gates before deleting (session-28 trap; bit again on `create_agent_task` / `normalize_orchestrator_agent_name`).
- **Trait impls are invisible to symbol greps.** Deleting a file with `impl From<X> for Y` / `impl Trait for Z` can orphan a sibling that needed the impl. Build after deleting any file containing trait impls (caught `From<AIAgentHarness> for Harness` via E0308).
- **Shared callbacks / sinks.** A handler reached by both a dead path and a live path (`on_workspaces_updated`, `toggle_share_dialog`) — verify it has NO live caller before removing; otherwise collapse only the dead feeder.
- **Linker-dead ≠ shrinks now.** Deleting a module that was already dead-stripped yields ~0 binary delta; the real shrink lands when the last LIVE-LINKED consumer (a registered singleton, a `TypedActionView`, a diesel `persist()` arm, a graphql op-builder) goes. Don't be surprised by a flat delta on a big LoC removal — and DON'T assume zero either; measure.

## Gate matrix (run all, in parallel, each own target dir)
```bash
CARGO_TARGET_DIR=target/gate-default cargo check -p warp
CARGO_TARGET_DIR=target/gate-tests   cargo check -p warp --tests
CARGO_TARGET_DIR=target/gate-feat    cargo check -p warp --features local_fs,gui
```
Background each in one message; wall-clock ≈ slowest gate. **Cross-crate**: `-p warp` does NOT cover `warp_cli`, `remote_server`, `warp_server_client`, `managed_secrets`. If you touched a shared crate or swept FeatureFlag variants, also gate the affected crate (`cargo check -p remote_server --tests`, etc.). The integration crate is not in the 3-gate.

Dead-code safety = the **3-gate intersection by message text**: an item dead in default+feat but referenced in `--tests` needs its test deleted too. cargo *warnings* ≠ the real removable set — verify callers, don't trust the warning list alone.

## Increment ordering
- **Consumers before the core.** Remove the feature's UI/actions/event-handlers first; the data-model keystone collapses last when its external refs hit zero.
- **Independent/clean-dead first, entangled later.** Land the unambiguous dead cut (e.g. a whole dead client trait + its file) before the risky core-model branch-collapses.
- **App-side before daemon/other-crate.** Sever the app caller, then neuter the daemon handler (keep wire-compat with an error response), then drop the now-dead client wrapper in the other crate (its own gate).
- **One concern per commit**, green each. A 30-file mechanical fan-out (test mocks) is fine in one commit; a core-model signature change + its fan-out is fine; mixing two unrelated removals is not.

## Tooling
- **Map first for anything entangled.** Dispatch a read-only investigator to produce a `file:line` classification table (DELETE / COLLAPSE / KEEP-reason) before cutting a core model. Cheaper than a red push + revert. Re-verify its "live" claims (investigators over-call "live"; they also occasionally mis-call dead).
- **recast for repetitive multi-file edits** (drop a ctor arg across 30 test files, remove an import everywhere, drop a struct-literal field). `recast_structural` for shape-sensitive; `recast_preview`→`recast_apply` for regex. Pass the explicit file list when the repo is large (`--max-files`). Single-pass arg-drop regexes are non-convergent (`mock(a,b)`→`mock(b)` re-matches) — pass `allow_non_convergent: true` for the one-shot.
- **Build fresh before claiming it compiles/runs.** `cargo run` reuses a stale binary; only trust output showing "Compiling warp". For UI/render-path or SSH/PTY changes, cargo-green ≠ works — build the GUI and eyeball it.

## After each strip commit
1. `cargo build --bin <binary> --features gui` (background) → `stat -c%s` the binary.
2. Add a row to the build-size log: bytes, MiB, MB, delta vs last row, and **whether the delta is real or linker-invisible and why**.
3. Update the tracker/plan + a per-session memory: what landed, what's KEEP-and-why, the next target, and any trap hit. Correct stale entries you found.

## Meta (workflow discipline)
- **No `while`/`until` poll loops in bash** — a wrong condition orphans background processes. Use a long fixed `timeout`, or wait on the harness completion-notification.
- **No `ScheduleWakeup` / `/loop` / `/schedule` / `Workflow` (multi-agent) without explicit user approval.** Single read-only investigator subagents and background builds are fine.
- **Don't claim done on a stub** — flag seams honestly. Don't report a flat binary as "removed" if only stubbed.
