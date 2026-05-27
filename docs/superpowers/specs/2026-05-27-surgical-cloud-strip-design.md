# Surgical cloud-strip of Warp — design

Date: 2026-05-27
Branch: `surgical-strip` (off upstream `0b737e22`)
Supersedes: `strip-cloud` branch (delete-foundation-first approach, abandoned)

## 1. Goal

Launch Warp's GUI with no cloud features. Same window, terminal blocks, settings,
themes, MCP, LSP, vim — minus login/auth, AI panel/agent, telemetry, cloud sync,
billing, teams, drive, shared sessions. Vendor agent CLIs (claude, codex, gemini,
aider) still run as ordinary shell processes inside.

Hard constraints (from the user):
- Clean tree. No dead code, no dormant phone-home code.
- The render path MUST work. NOT "a binary that compiles but renders nothing."
- AI stripping must be **surgical** — excise AI/cloud from the orchestrator, do not
  delete the orchestrator.

## 2. Why restart (root cause of the old branch)

The `strip-cloud` branch was built on three upstream commits:
- wave-1 `62ff7f94`: "rip Warp cloud / AI / login layer" — deleted the cloud **engine
  crates** (`ai`, etc.).
- wave-2 `61098c4f`: bulk import pruning.
- wave-3 `387dc542`: deleted `app/src/workspace/` (46 files, 54383 lines) including the
  main content orchestrator `workspace/view.rs` (25321 lines).

The structural mistake: **the foundation (`ai` crate) was deleted before its callers
were refactored.** Result — the branch has never compiled. It carried 4189 errors with
no green checkpoint to work against. With no compiler feedback loop, an over-aggressive
strip commit (`5645389a`) also gutted `root_view.rs` 3589 → 79 lines (the empty stub
that renders `Stack::new()` — blank window). Driving those 4189 errors to zero would
yield a binary that compiles and renders nothing — violating the hard constraint.

Surgical excision is far easier from a **compiling** base, removing one feature at a
time while keeping the tree green throughout — the inverse of wave-1.

## 3. P0 baseline — VALIDATED

`cargo check -p warp` on upstream `0b737e22` (default features, cloud included):
**0 errors, 46 warnings, 129 crates.** Confirmed green on this machine
(toolchain 1.92.0). This is the foundation the strip works against.

## 4. Architecture (what renders what)

- `RootView` (`app/src/root_view.rs`, 3588L upstream): window-level — tabs, quake mode,
  window open/restore, onboarding, auth gating (392 auth refs). Holds `Workspace`
  instances.
- `WorkspaceView` (`app/src/workspace/view.rs`, 25321L upstream): the **content
  orchestrator** — tab bar + pane layout + side panels + AI assistant panel. Cloud
  density: ~1361 AI/conversation refs, 457 agent, 269 cloud, 160 sync, 157 telemetry,
  131 auth, 54 shared_session.
- `PaneGroup` (`app/src/pane_group/mod.rs`): one tab's pane/terminal layout holding
  terminal sessions/blocks. Mostly local.

Both `RootView` and `WorkspaceView` are part of the render path and MUST survive the
strip (AI/cloud branches excised, render structure kept).

## 5. Cloud surface to remove

Engine crates (remove after callers refactored off them): `ai`, `firebase`, `graphql`,
`computer_use`, `managed_secrets`, `managed_secrets_wasm`, `onboarding`,
`warp_server_client`, `warp_graphql_schema`, `warp_web_event_bus`, `websocket`. (Audit
each — some may have local-only pieces worth keeping; decide case-by-case.)

Feature flags (lever): the `app` crate has a large `[features]` block; much cloud code
is `#[cfg(feature = ...)]`-gated (`agent_mode`, `viewing_shared_sessions`,
`creating_shared_sessions`, `agent_shared_sessions`, `crash_reporting`, …). Flipping a
flag off and deleting its dead `cfg` blocks is the cheapest excision.

UI/logic to remove: AI assistant panel + agent management, login/auth + cloud
onboarding, telemetry, cloud sync + drive, billing/usage, teams, shared sessions, cloud
code-review, computer-use.

## 6. Preserve (must keep rendering)

terminal + blocks + `PaneGroup`, `WorkspaceView` + `RootView` orchestrators, settings
UI, themes, MCP, LSP, vim, local workflows, command palette, local persistence (sqlite).

## 7. Execution phases — green after every commit

**Invariant: `cargo check -p warp` is green before each commit.** Never delete a
dependency before its callers are refactored off it.

- **P1 — Flip cloud feature flags off.** Remove cloud features from `default`; delete the
  now-dead `#[cfg(feature = ...)]` blocks. Compiler identifies exactly what's gated.
  Green + commit per feature.
- **P2 — Excise ungated cloud UI/logic, caller-side.** Per feature (AI panel, auth/login,
  telemetry, sync/drive, billing, teams, shared-sessions, code-review, computer-use):
  delete the panel/UI and refactor every caller off the cloud crate, **preserving
  `impl View` / `fn render` / `fn paint`** and the orchestrators. Green + commit per
  feature.
- **P3 — Delete cloud engine crates.** Once a crate has zero remaining callers, remove it
  from the workspace + `Cargo.toml`. Green + commit per crate.
- **P4 — Scrub.** Remove dormant flag defs, dead config keys, telemetry endpoints. Grep
  for phone-home (warp.dev HTTP, firebase, analytics). Verify no dead/dormant cloud code.
- **P5 — Launch.** `cargo build --bin warp-oss`, run the GUI, verify render: terminal
  blocks, settings, themes, MCP, LSP, vim. (Render is validated incrementally — it never
  breaks, because P0 starts green and the invariant holds.)

## 8. Branch & reference

- Work branch: `surgical-strip` off `0b737e22`, in worktree `~/Documents/git/warp-upstream`.
- `strip-cloud` kept untouched as reference — its seg1-3 refactors (remote_server,
  persistence, workflows) are reusable hints for the same subsystems here.
- Conventional commits: `strip:`, `refactor:`, `feat:`, `fix:`, `chore:`, `docs:`. No
  `Co-Authored-By` trailers. No `--no-verify`.

## 9. Risks & mitigations

- **Feature flags don't gate everything.** Auth/telemetry/sync are likely always-on.
  P2 handles ungated code explicitly; don't assume P1 catches it all.
- **Orchestrator coupling is deep** (WorkspaceView 1361 AI refs). Excise per-feature with
  the render-preservation guardrail; one focused pass per feature, never parallel on the
  orchestrator.
- **A cloud crate may host local-only types.** Audit before deleting (P3); relocate
  keepers to a local module rather than deleting them with the crate.
- **Scope creep into unrelated refactors.** Strip only; no opportunistic rewrites.

## 10. Done

`warp-oss` launches the Warp window — terminal blocks, settings, themes, MCP, LSP, vim —
with no login/telemetry/AI/cloud, no dormant phone-home code, render path working.
