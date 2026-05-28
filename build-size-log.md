# Build size log

Track GUI binary size over the strip. Goal: number trends down as cloud crates die.

Binary path: `target/debug/sublight` (was `target/debug/warp-oss` until the `a83cc3a5` rebrand)
Build command: `cargo build --bin sublight --features gui`
Profile: **debug** (release run separately if needed; note in row).

## Convention

One row per measured build. Add row when:
- A meaningful strip lands (crate deletion, feature gate removal, dead-code purge).
- Anytime you rebuild and the size shifts > ~5 MB.

Don't add a row for pure docs commits — size unchanged.

| Date       | Commit     | Profile | Step / milestone                                  | Bytes        | MiB   | MB (SI) | Build time | Notes |
|------------|------------|---------|---------------------------------------------------|--------------|-------|---------|------------|-------|
| 2026-05-28 | `c7319c0a` | debug   | Telemetry strip COMPLETE (cleanup-2)              | 914,048,408  | 871.7 | 914     | 2m08s      | First tracked datapoint — post-telemetry baseline. No pre-strip number captured. |
| 2026-05-28 | `7faa67bb` | debug   | Wasm orphans gone (serve-wasm, managed_secrets_wasm, warp_web_event_bus) | 914,048,408 | 871.7 | 914 | n/a (cached) | Zero binary delta — all three crates were wasm-only or wasm-served; never linked into the linux GUI build to begin with. Cleanup is repo hygiene, not size reduction. |
| 2026-05-28 | `007c18e8` | debug   | Onboarding crate deleted; login-slide seam relocated into app/src/auth/ | 913,583,608 | 871.3 | 913.6 | full rebuild | −464,800 B (−454 KiB) vs prior. ~10.6k LoC of slide/visual/callout code removed; small binary impact because most onboarding code paths were already dead-stripped by the linker — only metadata/symbol residue shrinks. |
| 2026-05-28 | `242147d3` | debug   | crash_reporting + sentry + cocoa_sentry + heap_usage_tracking stripped | 913,581,248 | 871.3 | 913.6 | full rebuild | −2,360 B (−2 KiB) vs prior. Near-zero binary delta because all four features were OFF by default — Sentry SDK + transitive deps were never linked into the default build. The real win is in Cargo.lock (~780 lines of sentry/ureq/etc transitive deps removed) and build-graph hygiene. |
| 2026-05-28 | `6d4b4a14` / `1f946e6f` | debug   | Tier-D rebrand: `warp-oss` → `sublight` (binary, bundle metadata, user-facing strings) + README + brand/ assets | 913,581,248 | 871.3 | 913.6 | full rebuild | Zero binary delta — rebrand touches strings + metadata only, no LoC changes. Binary now lands at `target/debug/sublight`. The old `target/debug/warp-oss` artifact is stale (cargo doesn't sweep renamed bins; `cargo clean` removes it). |
| 2026-05-29 | `f6789b13` | debug   | All warnings cleared (113 → 0) via batch of post-strip dead-code sweeps + 2 real cascade strips replacing earlier `#[allow(dead_code)]` cheats | 913,256,408 | 870.9 | 913.3 | full rebuild | −324,840 B (−317 KiB) vs prior measured. Cumulative session delta over ~20 commits in the warning-clearing sweep: channel bins, LoginSlide subtree, PasteAuthTokenModal, Sentry SDK, 17 dead FeatureFlag variants, ~60 dead telemetry types, ResourceUsageReporter pipeline (~427 LoC), RequestFileEditsFormatKind cascade (~73 LoC). The bulk of the cumulative LoC drop didn't translate to a proportional binary drop — most was already linker-stripped; only debug symbols / metadata shrink. |

## Next milestones to log

- Firebase + experiments + wasm crate deletions
- Login pass (delete `crates/onboarding`)
- AI crate deletion
- Cloud / billing / sharing / teams / Oz removal
- crash_reporting + profiling + analytics feature flags
- Release-profile baseline (build once with `--release` and log alongside)

## Why debug, not release

Debug builds fast enough to measure after each step. Release is the user-facing number — capture it at major milestones (telemetry done = NOW would be a good one; defer until end of next major pass).
