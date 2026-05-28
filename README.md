<p align="center">
  <img width="480" alt="Sublight" src="brand/sublight-wordmark.svg" />
</p>

<p align="center">
  Local-only terminal — a personal fork of <a href="https://github.com/warpdotdev/warp">warpdotdev/warp</a> with the cloud surfaces stripped out.
</p>

## What this is

Sublight keeps the parts of Warp that work without a network: the window, terminal blocks, settings, themes, MCP, LSP, vim emulation. It rips out the parts that depend on a hosted backend: telemetry, crash reporting, the AI agent panel, login/auth, billing, Warp Drive (cloud sync), session sharing, teams, Oz.

Vendor coding agents (Claude Code, Codex, Gemini CLI, aider, …) still run as ordinary shell processes inside Sublight — they just aren't tied into a hosted agent panel any more.

This is not affiliated with or endorsed by Warp / Denver Technologies, Inc.

## Status

Work in progress. The strip is being done feature-by-feature on the `surgical-strip` branch, keeping `cargo check -p warp` at zero errors after every commit. See [`plan.md`](plan.md) for the current state of each surface and [`build-size-log.md`](build-size-log.md) for how the binary size moves as the cloud surfaces are removed.

Done so far: telemetry pipeline, Sentry crash reporting, the onboarding flow + crate, three wasm helper crates, the FreeUserNoAi experiment, and the user-facing rebrand to Sublight. Next up: Firebase + the auth/server backend.

## Building

```bash
cargo run --bin sublight --features gui
```

On Linux, system dependencies are the usual `wgpu` set (Wayland or X11, Vulkan, fontconfig, etc.). Do **not** run `./script/bootstrap` — it's Debian/`apt`-only and assumes the upstream Warp build environment.

The first `--features gui` build is long because it pulls in the full GUI stack.

## Layout

- `app/` — the Sublight application crate (entry point, windowing, terminal model, settings, MCP, LSP, vim, …).
- `crates/` — supporting libraries, most of them named `warp_*` (preserved upstream attribution).
- `plan.md` — strip plan, status table, and lessons.
- `build-size-log.md` — debug-binary size tracked across the strip.
- `brand/` — Sublight brand assets.

## License

The repository tracks the original Warp license layout:

- `warpui_core` and `warpui` crates remain under the [MIT license](LICENSE-MIT).
- Everything else is under the [AGPL v3](LICENSE-AGPL).

Per AGPL §5, this is a **modified version** of Warp. The original work is © Denver Technologies, Inc.; modifications in this fork are © Sublight contributors. Sublight is not the same software as Warp and is not endorsed by Denver Technologies.

## Upstream credit

Sublight is built on top of [warpdotdev/warp](https://github.com/warpdotdev/warp). All terminal-block / agent-mode / wgpu-UI engineering is upstream's work. This fork only removes things — the goal is to keep the local experience while severing the hosted-product dependencies.
