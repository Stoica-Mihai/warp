# Build size log

Track GUI binary size over the strip. Goal: number trends down as cloud crates die.

Binary path: `target/debug/warp-oss`
Build command: `cargo build --bin warp-oss --features gui`
Profile: **debug** (release run separately if needed; note in row).

## Convention

One row per measured build. Add row when:
- A meaningful strip lands (crate deletion, feature gate removal, dead-code purge).
- Anytime you rebuild and the size shifts > ~5 MB.

Don't add a row for pure docs commits — size unchanged.

| Date       | Commit     | Profile | Step / milestone                                  | Bytes        | MiB   | MB (SI) | Build time | Notes |
|------------|------------|---------|---------------------------------------------------|--------------|-------|---------|------------|-------|
| 2026-05-28 | `c7319c0a` | debug   | Telemetry strip COMPLETE (cleanup-2)              | 914,048,408  | 871.7 | 914     | 2m08s      | First tracked datapoint — post-telemetry baseline. No pre-strip number captured. |

## Next milestones to log

- Firebase + experiments + wasm crate deletions
- Login pass (delete `crates/onboarding`)
- AI crate deletion
- Cloud / billing / sharing / teams / Oz removal
- crash_reporting + profiling + analytics feature flags
- Release-profile baseline (build once with `--release` and log alongside)

## Why debug, not release

Debug builds fast enough to measure after each step. Release is the user-facing number — capture it at major milestones (telemetry done = NOW would be a good one; defer until end of next major pass).
