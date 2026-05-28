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

Done so far (all green on the 3-gate matrix):
- Welcome/get-started panes, onboarding app flow — app launches straight to a terminal.
- Settings-import regression fix (over-stubbed in the onboarding pass; restored as a standalone command).
- **Telemetry — STRIP COMPLETE.** No queue, no dispatcher, no sender, no payload structs, no macros, no traits, no event types. ~13.8k LoC total across step 1 (no-op macros) → step 2 (kill live send path) → step 3 (delete central enum, −6966 LoC) → step a (16 satellite enums) → step e (cargo fix warning sweep 795→142) → step b (warp_core traits + register macro + EnablementState) → step c (TelemetryCollector + TelemetryApi + rudder_message + telemetry_ext + secret_redaction + context + AppTelemetryContextProvider) → step d (lib.rs bootstrap wiring) → cleanup-1 (819 macro call-sites + 5 macro defs + import sweep, −5802 LoC across 193 files) → cleanup-2 (43 dead payload structs in events.rs). 42 surviving payload items are plain data carriers for non-telemetry features (PaletteSource, CLIAgentType, AIAgentInput, etc.). Last commit: `c7319c0a`. Verified GUI binary builds (`cargo build --bin warp-oss --features gui` = 0 errors, 2m08s, 914 MB binary).
- **Wasm-orphan crates — STRIP COMPLETE.** Three crates gone: `serve-wasm` + `managed_secrets_wasm` (zero fan-in, `029a7e17`), `warp_web_event_bus` (two `cfg(target_family="wasm")` consumers neutered, `7faa67bb`). Combined −398 LoC. Binary size unchanged (wasm crates were never linked into the linux GUI build). Firebase deferred — its only callers live inside `server_api/auth.rs`, so it deletes cleanly only as part of the login pass. Experiments deferred — `ServerExperiment` reaches into `persistence/sqlite.rs` (KEPT for session restore) and needs a separate persistence-shape investigation before deletion.
- **Onboarding crate deleted (`007c18e8`).** 4-item login-slide seam relocated into `app/src/auth/` (`login_slide_layout.rs` + `login_slide_content.rs` as siblings, plus `OnboardingIntention` / `AI_FEATURES` / `WARP_DRIVE_FEATURES` inlined into `login_slide.rs`). `pub use` chain in `terminal/view/action.rs` re-pointed. −10,599 net LoC. Binary 914.0 → 913.6 MB (−454 KiB).
- **Onboarding feature-flag + bundle scrub (`992dc39e`).** `FeatureFlag::{AgentOnboarding, HOAOnboardingFlow}` + 4 Cargo.toml feature lines + 5 `assets/onboarding` bundle entries. −21 LoC.
- **FreeUserNoAi experiment + flag (`996760cc`).** Zero external callers, full chain dropped (server-experiment variants + match arms + `is_free_user_no_ai_experiment_active` + `FeatureFlag::FreeUserNoAi`). −31 LoC.
- **Sentry crash-reporting fully stripped (`242147d3`).** crash_reporting + cocoa_sentry + heap_usage_tracking + log_expensive_frames_in_sentry features gone; sentry/sentry-log/minidumper/crash-handler deps gone; `app/src/crash_reporting/` dir gone; 22 cfg sites collapsed; macOS Sentry.xcframework download + bundle entries gone; `FeatureFlag::{CocoaSentry, CrashReporting, LogExpensiveFramesInSentry}` gone. 32 files, −2,615 LoC source + −780 lines Cargo.lock. Binary near-zero delta (features were OFF by default); win is in build graph + audit surface. `PrivacySettings.is_crash_reporting_enabled` kept for the privacy/auth pass.
- **Dead telemetry residue (`7803fedc`).** `crates/ai/src/telemetry.rs` deleted (dead `CodebaseContextSyncType` enum) + 6 unused imports in events.rs. Warnings 120 → 113.
- **Tier-D rebrand to Sublight (`6d4b4a14` + `1f946e6f`).** Binary `warp-oss` → `sublight`; bundle metadata + embedded macOS Info.plist + AppId + WINDOW_TITLE + 5 toast titles + macOS app menu + "About Warp" command all switched. README rewritten for Sublight with AGPL §5 modified-warp disclaimer and upstream attribution; `brand/` assets (wordmark, mark, icon, favicons) landed. Internal `warp_*` library crates intentionally kept (honest upstream attribution).
- **Upstream channel bins dropped (`b847ec27`).** stable/preview/dev/local/integration `[[bin]]` decls + the 6 .rs files (incl. channel_config.rs) + 4 dead `bundle.bin.*` sections. Sublight doesn't ship through Warp's release pipeline. −352 net LoC. `Channel::{Stable, Preview, Dev, Local, Integration}` enum variants stay alive (referenced by ~30 autoupdate / appearance / auth_state / etc. sites) — pruning them is a separate cascade.
- **LoginSlideView subtree deleted (`0afda117`).** Legacy onboarding-era login slide with zero remaining external constructors. Live login UI is the unrelated AuthView. Deletes login_slide.rs + login_slide_layout.rs + login_slide_content.rs (the latter two had been needlessly relocated into auth/ at `007c18e8` to preserve a seam that fed nothing). −1812 LoC. Warnings 113 → 102.
- **Channel::Oss string rebrand (`68321755`).** `cli_command_name`, `Display`, `url_scheme`, and `ChannelState::init()` AppId all switch from `warp-oss` / `warposs` / `("dev","warp","WarpOss")` to `sublight` / `("local","sublight","Sublight")`. Closes a gap left by the binary rename at `6d4b4a14`.
- **Dead FeatureFlag sweep (`de2b7e0c` + `8d50a54e`).** 17 variants with zero readers anywhere across the workspace. −34 LoC.
- **Dead-telemetry residue across 5 files (`40808ba5` + `dc6613d2` + `88c7dc81` + `d822c832`).** Pruned post-strip orphan event-payload types in request_file_edits/, ai/blocklist/, ai/agent_management/, ai/ambient_agents/, code/lsp_telemetry.rs (whole file gone), code_review/telemetry_event.rs. −424 LoC. Warnings 102 → 68.

**Next**: firebase + auth/server layer strip (the big remaining cloud surface). Other parked items: full Tier-C library-crate rename (`warp_*` → `sublight_*`); privacy/auth UI consolidation; broader autoupdate strip (3680 LoC across 8 files, but high callgraph fan-in so needs careful sequencing); `Channel` enum variant cascade (attempted this session and reverted — produces 35+ cascade errors across ~15 files including autoupdate/* and remote_server/setup.rs, and most consumers are inside modules slated for removal anyway, so doing this cascade now = wasted work).

Telemetry-strip techniques that worked (detail in `plan.md`): no-op the send-macros first so their args aren't type-checked → event enums delete without touching the ~819 call-sites; recast (regex non-greedy `(?s)NAME!\(.*?\);` matched all invocations in 1 sweep — but `\b` matched starting at the macro name and left `crate::` prefixes orphaned, requiring a follow-up sweep); `cargo fix` for bulk unused removal but **re-verify gates** (it drops `#[cfg(test)]`/other-feature-only imports — broke `--tests` twice and dropped `permissions::CommandExecutionPermissionAllowedReason` re-export needed by a test); satellite modules are MIXED (event enum + helper types real code uses) → surgically delete enum+impls+`register_telemetry_event!`, keep helpers; the events.rs payload-struct prune needs intra-file ref tracking (items only referenced by other items in the same file need cascade or a graph walk).

## 4. Build / verify

- Build + launch GUI: `cargo run --bin sublight --features gui`. Do **not** run `./script/bootstrap` (Debian/apt-only; on this CachyOS box the deps are already present). First `--features gui` build is long. Binary lands at `target/debug/sublight`.
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
