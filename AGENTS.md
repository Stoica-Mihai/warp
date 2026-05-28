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

**`plan.md` (this directory) is the single source of truth** for status, per-finding difficulty, removal order, resume notes, and lessons. Read it first. Strategy detail: `docs/superpowers/specs/2026-05-27-surgical-cloud-strip-design.md`. Keep §1–§2 here synced with `plan.md`; let `plan.md` hold everything volatile.

Done so far (all green on the 3-gate matrix):
- Welcome/get-started panes, onboarding app flow — app launches straight to a terminal.
- Settings-import regression fix (over-stubbed in the onboarding pass; restored as a standalone command).
- **Telemetry — STRIP COMPLETE.** No queue, no dispatcher, no sender, no payload structs, no macros, no traits, no event types. ~13.8k LoC total across step 1 (no-op macros) → step 2 (kill live send path) → step 3 (delete central enum, −6966 LoC) → step a (16 satellite enums) → step e (cargo fix warning sweep 795→142) → step b (warp_core traits + register macro + EnablementState) → step c (TelemetryCollector + TelemetryApi + rudder_message + telemetry_ext + secret_redaction + context + AppTelemetryContextProvider) → step d (lib.rs bootstrap wiring) → cleanup-1 (819 macro call-sites + 5 macro defs + import sweep, −5802 LoC across 193 files) → cleanup-2 (43 dead payload structs in events.rs). 42 surviving payload items are plain data carriers for non-telemetry features (PaletteSource, CLIAgentType, AIAgentInput, etc.). Last commit: `c7319c0a`. Verified GUI binary builds (`cargo build --bin warp-oss --features gui` = 0 errors, 2m08s, 914 MB binary).

**Next**: firebase + experiments + wasm crates (EASY). Then login pass (relocate ~470 LoC from `crates/onboarding` into `auth/`, delete the crate). README rebrand parked.

Telemetry-strip techniques that worked (detail in `plan.md`): no-op the send-macros first so their args aren't type-checked → event enums delete without touching the ~819 call-sites; recast (regex non-greedy `(?s)NAME!\(.*?\);` matched all invocations in 1 sweep — but `\b` matched starting at the macro name and left `crate::` prefixes orphaned, requiring a follow-up sweep); `cargo fix` for bulk unused removal but **re-verify gates** (it drops `#[cfg(test)]`/other-feature-only imports — broke `--tests` twice and dropped `permissions::CommandExecutionPermissionAllowedReason` re-export needed by a test); satellite modules are MIXED (event enum + helper types real code uses) → surgically delete enum+impls+`register_telemetry_event!`, keep helpers; the events.rs payload-struct prune needs intra-file ref tracking (items only referenced by other items in the same file need cascade or a graph walk).

## 4. Build / verify

- Build + launch GUI: `cargo run --bin warp-oss --features gui`. Do **not** run `./script/bootstrap` (Debian/apt-only; on this CachyOS box the deps are already present). First `--features gui` build is long.
- Verify green: 3-gate matrix — `cargo check -p warp` + `--tests` + `--features local_fs,gui`. Background them in parallel — they're slow.
- LSP works on this worktree, but injected diagnostics lag edits (stale line numbers). Trust `cargo check`, not the diagnostic stream.

## 5. Conventions

Conventional commits (`refactor:`/`feat:`/`fix:`/`docs:`/`chore:`). No `Co-Authored-By` trailers. No `--no-verify`. Commit only at green checkpoints. Don't claim done on a stub — flag seams honestly.

## 6. Multi-file rewrites — use `recast` MCP tools

This removal loop is one shape repeated across many sites — exactly recast's win zone. The `recast` MCP server is wired globally; prefer its tools over `Edit`/`sed` loops when the same syntactic change lands in many files.

- **5+ sites, simple text change** (rename, drop a feature flag, swap an import): `recast_preview` → inspect diff → `recast_apply`. 1-4 isolated sites: `Edit` is fine.
- **Shape-sensitive change** (remove an enum variant, add/drop a struct field, change a fn signature across callers): `recast_structural` with `ast_pattern` (`lang: "rust"`). Regex on AST shapes is fragile.
- **0 matches** = pattern wrong. Iterate the pattern; do NOT fall back to per-file `Edit`.
- **Footgun:** `replacement` is a regex template, not a C string. `\n` / `\t` are NOT decoded — they land as literal backslash-n on disk. Put a real newline in the JSON value. Backrefs `$1` / `${name}` ARE interpolated.

Atomic two-phase commit + rollback means a half-applied removal can't leave the tree uncompilable mid-batch — fits the green-always invariant.
