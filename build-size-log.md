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
| 2026-05-29 | `e2aeb960` | debug   | Autoupdate pipeline gone (dir + state machine + UI + feature flags + WorkspaceAction variants + bindings) | 911,714,648 | 869.5 | 911.7 | full rebuild | −1,541,760 B (−1.47 MB / −1505 KiB) vs prior — first strip this session with a non-trivial binary shrink. The autoupdate module had a live consumer chain (RootView polling → server_time → banner) that the linker couldn't dead-strip, so deleting it actually moved the needle. ~3680 LoC autoupdate dir + ~700 LoC of callers + feature flags + actions = −4382 LoC net source. |
| 2026-05-29 | `d58d199e` | debug   | Channel enum collapsed to single Oss variant + 30-site cascade collapse | 911,607,752 | 869.4 | 911.6 | full rebuild | −106,896 B (−104 KiB) vs prior. Small binary delta — most pruned code was already in dead branches the linker could see (string match arms returning per-channel constants). The win is in clarity + further unblocking auth/server strip: ChannelState now has no channel-discrimination logic, just app-id/server-url/etc constants. 32 files, −435 LoC. |
| 2026-05-29 | `144639a4` | debug   | AI agent_sdk + warp_cli::share session-sharing decouple (first step of session-sharing strip) | 911,327,448 | 869.1 | 911.3 | 2m13s | −280,304 B (−274 KiB) vs prior. Real shrink — the agent driver's sharing path (ShareSessionError, should_share/wait_for_session_shared, EstablishedSharedSession event, write_session_joined) + the `warp_cli::share` module/`--share` flag were live-linked, so the linker couldn't dead-strip them pre-removal. 13 files, −699 LoC. Confirms commit runtime-clean (full GUI link, not just cargo check). terminal/shared_session still mounted; the large atomic collapse is the next step. |
| 2026-05-29 | `bfcb3ffe` | debug   | **Session-sharing strip COMPLETE** (collapse both shared_session dirs to a ~95-line stub + delete machinery/share-UI across the codebase) | 899,392,512 | 857.7 | 899.4 | 1m45s | **−11.93 MB (−11,935,232 B) vs prior — by far the largest single shrink of the project.** The session-sharing machinery (~21.5k LoC across the engine dir + view-layer dir) + the sharer-network hub in terminal_manager + the share-UI woven through view.rs/pane_group/pane_impl/workspace/drive were all *live-linked* (mounted singletons, active event subscriptions, rendered UI), so the linker couldn't dead-strip any of it pre-removal. 94 files, **−27,714 net LoC**. SharedSessionStatus collapsed to a vestigial NotShared stub (111 read-sites neutered, not removed — deferred). 3-gate matrix green, 0 warnings. |
| 2026-05-29 | `96207e43` | debug   | **Warp Drive server-sync amputation COMPLETE** (keep local sqlite store, cut server leg) | 896,883,888 | 855.3 | 896.9 | 1m30s | **−2.51 MB (−2,508,624 B) vs prior.** 22 files, **−13,140 net LoC** — but ~11.4k of that is test code (update_manager_tests 7507, syncer_tests 1752, sync_queue_tests 1536, fake_object_client 465, test_utils 109) which never linked into the binary. The binary delta comes from the *live-linked* deletions: the inbound sync-merge cluster (on_changed_objects_fetched + handlers) + poll engine + cloud_preferences_syncer + the MCP-gallery server-push path. The server-action methods (trash/share/guests) + ObjectClient trait + server_api CRUD + SyncQueue enqueue are KEPT (still UI-reachable) — they go with the drive-UI / sharing pass, where the next Drive shrink will land. 3-gate matrix 0/0. |

## Next milestones to log

- Firebase + experiments + wasm crate deletions
- Login pass (delete `crates/onboarding`)
- AI crate deletion
- Cloud / billing / sharing / teams / Oz removal
- crash_reporting + profiling + analytics feature flags
- Release-profile baseline (build once with `--release` and log alongside)

## Why debug, not release

Debug builds fast enough to measure after each step. Release is the user-facing number — capture it at major milestones (telemetry done = NOW would be a good one; defer until end of next major pass).
