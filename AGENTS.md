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

telemetry · wasm-orphan crates · onboarding crate + flags · Sentry crash-reporting · Sublight rebrand · channel enum cascade · autoupdate pipeline · session-sharing (all vestiges incl. `SharedSessionStatus` + `is_shared_session_viewer` cascade) · Warp Drive server-sync + SyncQueue + ObjectClient · SharingDialog + permission-CRUD · login gate bypass · auth gate UI · AuthManager login stub · Firebase crate · server-driven A/B experiments · dead AuthClient privacy-sync · AuthView/AuthOverrideWarningModal cloud-gate UI · billing/teams/platform/referrals pages · AI assistant panel + Warp AI command search · AI settings pages + execution profile editor · execution profiles data model + inline profile selector · ScheduledAgentManager registration + AgentSource computation · **Phase C: `ai/agent_sdk/` STRIP COMPLETE** (`d3eb301d` + `889d83dd`)

**Current state:** Binary **821.9 MB** (−21.76 MB vs prior). **0 errors / ~180 warnings** (all AI territory, deferred, no `#[allow(dead_code)]` cheats). App always starts in Terminal.

**Phase C done:** `ai/agent_sdk/` (73 files, 31,891 LoC) deleted. 10 external caller files fixed:
- `ai/mod.rs`: drop `pub mod agent_sdk;`
- `lib.rs`: collapse `LaunchMode::CommandLine` arm (agent_sdk::run removed, exits with error message now)
- `ai/blocklist/controller.rs`: drop `ClaudeHarness` + `LocalClaudeWakeTrigger` + `pending_local_claude_wakes` + entire dormant-wake machinery; simplify `handle_pending_events_ready` to call inject directly; remove `DormantClaudeWakeReady` subscriber
- `ai/blocklist/handoff/snapshot.rs`: stub Local upload target → `EmptyWorkspace`
- `ai/blocklist/action_model/execute/upload_artifact.rs`: stub execute → `Error`
- `remote_server/handoff_snapshot.rs`: stub gather_and_upload → `Ok(None)`
- `server/server_api/harness_support.rs`: inline `with_bounded_retry` → direct calls
- `terminal/view/docker_sandbox/mod.rs`: stub `initialize_docker_sandbox_environment` → no-op; drop all agent_sdk/cloud_environment driver imports
- `terminal/view/ambient_agent/view_impl.rs`: remove `auth_check_command_for` block
- `pane_group/pane/local_harness_launch.rs`: rewrite without agent_sdk; keep command builders; set `ANTHROPIC_MODEL` env var directly; remove agent_sdk platform validation/harness_kind/task_env_vars

**Next — AI blocklist + agent strip (Phase D):** `ai/blocklist/`, `ai/agent/`, `ai/agent_management/`. Pre-existing cascade warnings clear naturally as construction sites disappear. `context_chips` NOT a target — core terminal prompt rendering (git branch, dir, virtualenv).

**Deferred (Phase B ambient-agents):** `ai/ambient_agents/` directory, `terminal/view/ambient_agent/` directory, `LeafContents::AmbientAgent` variant, ambient agent pane management in `pane_group/mod.rs`, GitHub OAuth environment management, `workspace/auto_handoff.rs`, `pane_group/pane/environment_management_pane.rs`. These can now proceed since Phase C is done.

**Also deferred:** `IsSharedSessionCreator` (used in child_agent.rs + pane_group + terminal_pane + terminal_manager + docker_sandbox via `SharedSessionSource`/`inherit_share_for_local_child`; remove with child_agent seam). `session_sharing_protocol` crate **KEEP** — `SharedSessionSource` + `SessionSourceType` load-bearing for ambient agents. `referral_theme_status.rs` **KEEP** — woven into theme_chooser + GlobalResourceHandles.

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
