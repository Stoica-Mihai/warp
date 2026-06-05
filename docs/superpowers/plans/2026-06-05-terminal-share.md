# Self-Hosted Terminal Share (partyserver relay) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Share terminal" action to the fork that mints a live URL; opening it in any browser gives real-time read **and** write access to that terminal session; un-sharing (or session end) kills the URL. No Warp cloud, no `cloudflared` binary, no stdout scraping — the host speaks plain outbound WebSocket to a self-owned Cloudflare Worker relay built on **partyserver**.

**Architecture:** Three decoupled subsystems joined by one dumb byte protocol.
1. **Relay** — a Cloudflare Worker using `partyserver`. Each share is a room (Durable Object) keyed by an unguessable id. The relay fans PTY-output bytes from the single *host* connection out to all *viewer* connections, and forwards viewer keystrokes/resize back to the host. The relay never interprets terminal content — it pipes opaque frames. Built-in WebSocket Hibernation keeps idle shares free.
2. **Viewer** — a static xterm.js page served by the same Worker at `/s/:id`. Connects to its room with `partysocket`; binary frames → `term.write`, `term.onData` → binary frames; sends resize on viewport change.
3. **Host** — a Rust crate inside the fork. Opens an outbound `wss://` to the relay with `tokio-tungstenite` (already vendored as `tungstenite`/`async-tungstenite`), taps the existing PTY (`app/src/terminal/writeable_pty/pty_controller.rs`), pumps output → relay and relay-input → `write_bytes` / `resize_pty`. `share()` returns the URL as a value; `unshare()` drops the socket → URL dies.

The host holds the connection object, so "is it live / online" is just "is the socket open" — fully in-process, no subprocess, no log parsing.

**Tech Stack:** TypeScript + `partyserver` + `partysocket` + xterm.js on Cloudflare Workers/Durable Objects (`wrangler` for dev/deploy). Rust + `tokio` + `tokio-tungstenite` + the fork's existing GPUI/`ModelContext` terminal stack on the host side.

---

## Scope & subsystem boundaries

This plan covers three subsystems that each build and test independently:

- **Phase 1 — Relay** (`relay/`): self-contained Worker, testable with `wrangler dev` + a Node WS client. Ships value alone (a generic terminal-share relay).
- **Phase 2 — Viewer** (`relay/public/`): static page, testable in a browser against the dev relay.
- **Phase 3 — Host crate** (`crates/terminal_share/`): standalone-testable Rust crate that shares a real shell against the dev relay, before any fork UI wiring.
- **Phase 4 — Fork integration**: replace the crate's own spawned shell with a tap into the live `pty_controller`, wire the Share/Unshare UI action and URL surfacing.
- **Phase 5 — Hardening**: config, auth secret, reconnect/backoff, view-only mode, and the e2e-encryption follow-up hook.

Phases 1–3 have **zero** dependency on the fork compiling and should be done first against `wrangler dev`. Phase 4 is the only part that touches the (heavy) Warp workspace.

---

## Wire protocol (shared by all three subsystems)

One protocol, deliberately dumb. Two frame kinds on a single WebSocket:

- **Binary frame** = raw PTY bytes. Host→relay = terminal output. Viewer→relay = keystrokes. Relay routes by sender role; never inspects bytes.
- **Text frame** = JSON control message:
  - `{"t":"hello","role":"host","secret":"<HOST_SECRET>"}` — first frame a host sends. Relay validates `secret`; rejects + closes (code 4401) on mismatch or if a host already owns the room.
  - `{"t":"hello","role":"viewer"}` — first frame a viewer sends.
  - `{"t":"resize","cols":<u16>,"rows":<u16>}` — viewer→host, relayed verbatim to host.
  - `{"t":"viewers","n":<u32>}` — relay→host, current viewer count (for UI badge). Sent on viewer join/leave.
  - `{"t":"bye"}` — graceful close (either side).

Routing rules in the relay:
- A binary frame from the **host** is broadcast to **all viewers**.
- A binary frame or `resize` from a **viewer** is forwarded **only to the host**.
- A viewer connecting with no live host in the room receives `{"t":"bye"}` and is closed (nothing to see when not shared).

Room id: 22+ char URL-safe random (host-minted). Viewer URL: `https://<relay-host>/s/<id>`. Host party URL: `wss://<relay-host>/parties/term/<id>?role=host` (the secret travels in the `hello` frame, never the URL/logs).

> **Encryption note:** Phase 1–4 relay plaintext (TLS to the Cloudflare edge, plaintext at the edge). Phase 5 Task 5.4 adds the optional e2e layer: a key in the viewer URL **fragment** (`#k=`, never sent to the server), host+viewer AES-GCM the binary payloads, relay keeps piping opaque blobs unchanged. Build the protocol so payloads are already opaque to the relay — Task 5.4 is then additive, not a rewrite.

---

## File structure

```
relay/                              # Phase 1 + 2  (Cloudflare Worker)
  package.json                      # partyserver, partysocket, wrangler, xterm
  wrangler.toml                     # DO binding "term" -> TermRoom, assets dir
  tsconfig.json
  src/
    server.ts                       # TermRoom extends Server; routing + auth + fan-out
    routes.ts                       # fetch(): routePartykitRequest || serveViewer
  public/
    s.html                          # viewer page (xterm.js + partysocket)
    viewer.ts                       # viewer logic (bundled to viewer.js)
  test/
    relay.test.ts                   # Node WS clients against `wrangler dev`

crates/terminal_share/              # Phase 3  (standalone-testable Rust crate)
  Cargo.toml
  src/
    lib.rs                          # public API: ShareHandle, share(), unshare()
    protocol.rs                     # Control enum (serde), frame encode/decode
    relay_client.rs                 # tokio-tungstenite outbound, pump loops
    pty_source.rs                   # trait PtySource { output stream; write; resize }
    standalone.rs                   # spawns a real shell via portable-pty (test/demo)
  tests/
    end_to_end.rs                   # crate <-> `wrangler dev` relay <-> fake viewer

# Phase 4  (fork integration — modify existing files)
app/src/terminal/writeable_pty/pty_controller.rs   # add output broadcast hook
app/src/terminal/...  (Share/Unshare action + URL surfacing — exact files via Task 4.1)
crates/terminal_share/src/fork_source.rs           # PtySource impl backed by pty_controller
```

---

## Phase 0: Investigation & project skeleton

### Task 0.1: Confirm the PTY output tap point in the fork

**Files:**
- Read: `app/src/terminal/writeable_pty/pty_controller.rs` (full, ~35.5K)
- Read: `app/src/terminal/writeable_pty/mod.rs`, `message.rs`
- Read: the terminal model that consumes PTY output (follow the event-loop thread referenced near `pty_controller.rs:659`/`:780`)

- [ ] **Step 1: Locate the output path.** `write_bytes` / `write_agent_bytes` / `resize_pty` are the confirmed **input** taps. Find where bytes read **from** the PTY master are dispatched (the event-loop thread that emits the "event loop thread has exited" message at `:780`). Record the exact function/struct and the type of the output payload (raw `Vec<u8>` vs already-parsed VT events).
- [ ] **Step 2: Decide the tap shape.** Prefer raw bytes. If output is only available post-VT-parse, the cleanest tap is to clone the raw byte buffer *before* it enters the parser. Document the chosen insertion point as `file:line` and the exact data type.
- [ ] **Step 3: Write findings** into this plan under Task 4.2 (replace the "VERIFY" markers there with the real symbols). No code yet — this is the contract Phase 4 depends on.

### Task 0.2: Scaffold the relay project

**Files:**
- Create: `relay/package.json`, `relay/wrangler.toml`, `relay/tsconfig.json`

- [ ] **Step 1: Init.**
```bash
mkdir -p relay/src relay/public relay/test && cd relay
npm init -y
npm i partyserver partysocket
npm i -D wrangler typescript @cloudflare/workers-types esbuild @types/node ws
```
- [ ] **Step 2: `wrangler.toml`** (Durable Object binding + static assets):
```toml
name = "term-share"
main = "src/routes.ts"
compatibility_date = "2026-06-01"

[[durable_objects.bindings]]
name = "Term"
class_name = "TermRoom"

[[migrations]]
tag = "v1"
new_sqlite_classes = ["TermRoom"]   # free plan requires SQLite-backed DO

[assets]
directory = "public"
binding = "ASSETS"
```
- [ ] **Step 3: `tsconfig.json`** — `target ES2022`, `module ES2022`, `moduleResolution bundler`, `types ["@cloudflare/workers-types"]`, `strict true`.
- [ ] **Step 4: Verify the toolchain runs.**
Run: `npx wrangler --version`
Expected: prints a wrangler version, no error.
- [ ] **Step 5: Commit.**
```bash
git add relay/package.json relay/wrangler.toml relay/tsconfig.json
git commit -m "chore(relay): scaffold partyserver worker project"
```

---

## Phase 1: Relay (partyserver Worker)

### Task 1.1: Room server — host/viewer roles + fan-out

**Files:**
- Create: `relay/src/server.ts`
- Test: `relay/test/relay.test.ts`

- [ ] **Step 1: Write the failing test** (`relay/test/relay.test.ts`). Drives the dev relay with raw `ws` clients: a host and two viewers; asserts host output reaches both viewers and viewer input reaches only the host.
```ts
import { test, before, after } from "node:test";
import assert from "node:assert";
import { WebSocket } from "ws";
import { spawn, type ChildProcess } from "node:child_process";

const HOST = "127.0.0.1:8787";
const SECRET = "test-secret";
let dev: ChildProcess;

const open = (u: string) => new Promise<WebSocket>((res, rej) => {
  const ws = new WebSocket(u); ws.once("open", () => res(ws)); ws.once("error", rej);
});
const next = (ws: WebSocket) => new Promise<any>((res) => ws.once("message", (d) => res(d)));

before(async () => {
  dev = spawn("npx", ["wrangler", "dev", "--port", "8787", "--local"], { cwd: "relay" });
  // wait until the port answers
  for (let i = 0; i < 60; i++) {
    try { const w = await open(`ws://${HOST}/parties/term/ping?role=viewer`); w.close(); break; }
    catch { await new Promise(r => setTimeout(r, 500)); }
  }
});
after(() => dev.kill("SIGTERM"));

test("host output fans out to viewers; viewer input goes only to host", async () => {
  const room = "room-" + Math.random().toString(36).slice(2);
  const host = await open(`ws://${HOST}/parties/term/${room}?role=host`);
  host.send(JSON.stringify({ t: "hello", role: "host", secret: SECRET }));

  const v1 = await open(`ws://${HOST}/parties/term/${room}?role=viewer`);
  v1.send(JSON.stringify({ t: "hello", role: "viewer" }));
  const v2 = await open(`ws://${HOST}/parties/term/${room}?role=viewer`);
  v2.send(JSON.stringify({ t: "hello", role: "viewer" }));
  await new Promise(r => setTimeout(r, 200));

  host.send(Buffer.from("OUT")); // host -> all viewers
  assert.deepEqual(Buffer.from(await next(v1)).toString(), "OUT");
  assert.deepEqual(Buffer.from(await next(v2)).toString(), "OUT");

  const gotByHost = next(host);
  v1.send(Buffer.from("IN")); // viewer -> host only
  assert.deepEqual(Buffer.from(await gotByHost).toString(), "IN");

  host.close(); v1.close(); v2.close();
});
```
- [ ] **Step 2: Run it to verify it fails.**
Run: `cd relay && node --test test/relay.test.ts`
Expected: FAIL (no `server.ts` routing yet / connection refused after timeout).
- [ ] **Step 3: Implement `server.ts`.**
```ts
import { Server, type Connection, type ConnectionContext } from "partyserver";

type Role = "host" | "viewer";
interface ConnState { role: Role }

const HOST_SECRET = "test-secret"; // overridden by env in Task 1.3

export class TermRoom extends Server {
  static options = { hibernate: true };

  onConnect(conn: Connection<ConnState>, ctx: ConnectionContext) {
    const role = (new URL(ctx.request.url).searchParams.get("role") ?? "viewer") as Role;
    conn.setState({ role });
    if (role === "viewer" && !this.host()) {
      conn.send(JSON.stringify({ t: "bye" }));
      conn.close(1000, "no host");
      return;
    }
    if (role === "host") this.broadcastViewerCount();
  }

  onMessage(conn: Connection<ConnState>, message: string | ArrayBuffer | ArrayBufferLike) {
    const role = conn.state?.role ?? "viewer";

    if (typeof message === "string") {
      let m: any; try { m = JSON.parse(message); } catch { return; }
      if (m.t === "hello") {
        if (role === "host" && m.secret !== HOST_SECRET) { conn.close(4401, "bad secret"); return; }
        if (role === "host") {
          const existing = this.host();
          if (existing && existing.id !== conn.id) { conn.close(4409, "host exists"); return; }
        }
        return;
      }
      if (m.t === "resize" && role === "viewer") { this.host()?.send(message); return; }
      if (m.t === "bye") { conn.close(1000, "bye"); return; }
      return;
    }

    // binary
    if (role === "host") {
      for (const v of this.viewers()) v.send(message);
    } else {
      this.host()?.send(message);
    }
  }

  onClose(conn: Connection<ConnState>) {
    if (conn.state?.role === "host") {
      for (const v of this.viewers()) { v.send(JSON.stringify({ t: "bye" })); v.close(1000, "host left"); }
    } else {
      this.broadcastViewerCount();
    }
  }

  private host(): Connection<ConnState> | undefined {
    for (const c of this.getConnections()) if (c.state?.role === "host") return c as Connection<ConnState>;
    return undefined;
  }
  private *viewers(): Iterable<Connection<ConnState>> {
    for (const c of this.getConnections()) if (c.state?.role === "viewer") yield c as Connection<ConnState>;
  }
  private broadcastViewerCount() {
    let n = 0; for (const _ of this.viewers()) n++;
    this.host()?.send(JSON.stringify({ t: "viewers", n }));
  }
}
```
> **VERIFY at build:** `getConnections()`, `conn.state`/`setState`, `static options.hibernate`, and `Connection`/`ConnectionContext` names against the installed `partyserver` README (`relay/node_modules/partyserver/README.md`). The shape is per the partyserver docs; adjust method names if the pinned version differs. This is a real verification step, not a placeholder.
- [ ] **Step 4: Add routing** in `relay/src/routes.ts` (needed for the test to connect):
```ts
import { routePartykitRequest } from "partyserver";
export { TermRoom } from "./server";

export default {
  async fetch(request: Request, env: Record<string, unknown>): Promise<Response> {
    return (await routePartykitRequest(request, env, { prefix: "parties" }))
      ?? new Response("not found", { status: 404 });
  },
};
```
- [ ] **Step 5: Run the test to verify it passes.**
Run: `cd relay && node --test test/relay.test.ts`
Expected: PASS (1 test).
- [ ] **Step 6: Commit.**
```bash
git add relay/src/server.ts relay/src/routes.ts relay/test/relay.test.ts
git commit -m "feat(relay): room fan-out with host/viewer roles"
```

### Task 1.2: Reject viewers when not shared + drop second host

**Files:**
- Modify: `relay/test/relay.test.ts`
- Modify: `relay/src/server.ts` (logic already drafted in 1.1 — this task locks it with tests)

- [ ] **Step 1: Write failing tests.** (a) viewer connecting to a room with no host receives `{"t":"bye"}` and is closed; (b) a second host gets close code 4409.
```ts
test("viewer with no host is rejected", async () => {
  const room = "empty-" + Math.random().toString(36).slice(2);
  const v = await open(`ws://${HOST}/parties/term/${room}?role=viewer`);
  v.send(JSON.stringify({ t: "viewer" }));
  const msg = JSON.parse(Buffer.from(await next(v)).toString());
  assert.equal(msg.t, "bye");
});

test("second host is rejected with 4409", async () => {
  const room = "dup-" + Math.random().toString(36).slice(2);
  const h1 = await open(`ws://${HOST}/parties/term/${room}?role=host`);
  h1.send(JSON.stringify({ t: "hello", role: "host", secret: SECRET }));
  await new Promise(r => setTimeout(r, 100));
  const h2 = await open(`ws://${HOST}/parties/term/${room}?role=host`);
  const code: number = await new Promise(res => { h2.once("close", c => res(c)); h2.send(JSON.stringify({ t: "hello", role: "host", secret: SECRET })); });
  assert.equal(code, 4409);
  h1.close();
});
```
- [ ] **Step 2: Run to verify.**
Run: `cd relay && node --test test/relay.test.ts`
Expected: PASS (3 tests). If a case fails, the branch is in `onConnect`/`onMessage` from Task 1.1 — fix there.
- [ ] **Step 3: Commit.**
```bash
git add relay/src/server.ts relay/test/relay.test.ts
git commit -m "test(relay): no-host rejection + single-host invariant"
```

### Task 1.3: Move host secret to env

**Files:**
- Modify: `relay/src/server.ts`, `relay/wrangler.toml`, `relay/test/relay.test.ts`

- [ ] **Step 1: Read secret from env.** Replace the module const with per-instance env access. partyserver exposes the Worker `env` on the server instance (`this.env`); read `this.env.HOST_SECRET`.
```ts
// in onMessage hello branch:
if (role === "host" && m.secret !== (this.env as any).HOST_SECRET) { conn.close(4401, "bad secret"); return; }
```
- [ ] **Step 2: Add dev var.** In `relay/wrangler.toml`:
```toml
[vars]
HOST_SECRET = "test-secret"
```
For production use `npx wrangler secret put HOST_SECRET` (not committed).
- [ ] **Step 3: Run tests.**
Run: `cd relay && node --test test/relay.test.ts`
Expected: PASS (3 tests).
- [ ] **Step 4: Commit.**
```bash
git add relay/src/server.ts relay/wrangler.toml
git commit -m "feat(relay): host secret from env"
```

---

## Phase 2: Viewer (xterm.js + partysocket)

### Task 2.1: Serve the viewer page at `/s/:id`

**Files:**
- Modify: `relay/src/routes.ts`
- Create: `relay/public/s.html`

- [ ] **Step 1: Route `/s/:id` to the page.** In `routes.ts`, before the 404, serve the static asset for any `/s/...` path (the room id is read client-side from the path):
```ts
const url = new URL(request.url);
if (url.pathname.startsWith("/s/")) {
  return (env.ASSETS as { fetch: (r: Request) => Promise<Response> })
    .fetch(new Request(new URL("/s.html", url), request));
}
```
- [ ] **Step 2: Minimal page** (`relay/public/s.html`) loading xterm from a CDN and a bundled `viewer.js` (built in Task 2.2):
```html
<!doctype html><meta charset="utf-8"><title>terminal</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@xterm/xterm/css/xterm.css">
<style>html,body,#t{height:100%;margin:0;background:#000}</style>
<div id="t"></div>
<script src="https://cdn.jsdelivr.net/npm/@xterm/xterm/lib/xterm.js"></script>
<script src="https://cdn.jsdelivr.net/npm/@xterm/addon-fit/lib/addon-fit.js"></script>
<script type="module" src="/viewer.js"></script>
```
- [ ] **Step 3: Verify it serves.**
Run: `cd relay && npx wrangler dev --port 8787 --local &` then `curl -s http://127.0.0.1:8787/s/anything | grep -o xterm | head -1`
Expected: prints `xterm`.
- [ ] **Step 4: Commit.**
```bash
git add relay/src/routes.ts relay/public/s.html
git commit -m "feat(relay): serve xterm viewer page at /s/:id"
```

### Task 2.2: Viewer logic — connect, render, type, resize

**Files:**
- Create: `relay/public/viewer.ts` (bundled → `relay/public/viewer.js`)
- Modify: `relay/package.json` (build script)

- [ ] **Step 1: Implement viewer.**
```ts
import { PartySocket } from "partysocket";
// xterm + FitAddon are global from the CDN script tags
declare const Terminal: any, FitAddon: any;

const id = location.pathname.split("/").pop()!;
const term = new Terminal({ cursorBlink: true, fontFamily: "monospace", convertEol: false });
const fit = new FitAddon.FitAddon();
term.loadAddon(fit);
term.open(document.getElementById("t"));
fit.fit();

const ws = new PartySocket({ host: location.host, party: "term", room: id, query: { role: "viewer" } });
ws.binaryType = "arraybuffer";

ws.addEventListener("open", () => {
  ws.send(JSON.stringify({ t: "hello", role: "viewer" }));
  sendResize();
});
ws.addEventListener("message", (e: MessageEvent) => {
  if (typeof e.data === "string") {
    const m = JSON.parse(e.data);
    if (m.t === "bye") { term.write("\r\n\x1b[31m[session ended]\x1b[0m\r\n"); ws.close(); }
    return;
  }
  term.write(new Uint8Array(e.data));
});

term.onData((d: string) => ws.send(new TextEncoder().encode(d)));
function sendResize() { ws.send(JSON.stringify({ t: "resize", cols: term.cols, rows: term.rows })); }
addEventListener("resize", () => { fit.fit(); sendResize(); });
```
- [ ] **Step 2: Build script** in `relay/package.json`:
```json
"scripts": { "build:viewer": "esbuild public/viewer.ts --bundle --format=esm --outfile=public/viewer.js" }
```
- [ ] **Step 3: Build + verify it bundles.**
Run: `cd relay && npm run build:viewer && test -s public/viewer.js && echo OK`
Expected: prints `OK`.
- [ ] **Step 4: Manual end-to-end smoke** (documented, run once): start `wrangler dev`, run a Node host that opens `?role=host`, sends `hello`+secret, then pipes `process.stdin`→binary and binary→`process.stdout`; open `http://127.0.0.1:8787/s/<room>` in a browser; confirm typing in the browser appears in the Node host and vice-versa. Capture a screenshot for the PR.
- [ ] **Step 5: Commit.**
```bash
git add relay/public/viewer.ts relay/public/viewer.js relay/package.json
git commit -m "feat(viewer): xterm.js + partysocket bidirectional client"
```

---

## Phase 3: Host crate (standalone, against dev relay)

### Task 3.1: Protocol types

**Files:**
- Create: `crates/terminal_share/Cargo.toml`, `crates/terminal_share/src/lib.rs`, `crates/terminal_share/src/protocol.rs`
- Test: inline `#[cfg(test)]` in `protocol.rs`

- [ ] **Step 1: Cargo.toml.**
```toml
[package]
name = "terminal_share"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-util", "sync", "time"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = "0.8"
anyhow = "1"

[dev-dependencies]
portable-pty = "0.8"
```
- [ ] **Step 2: Write the failing test** (`protocol.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_roundtrips() {
        let c = Control::Resize { cols: 120, rows: 40 };
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Control>(&s).unwrap(), Control::Resize { cols: 120, rows: 40 });
    }
    #[test]
    fn hello_host_has_secret() {
        let s = serde_json::to_string(&Control::Hello { role: Role::Host, secret: Some("x".into()) }).unwrap();
        assert!(s.contains("\"secret\":\"x\""));
    }
}
```
- [ ] **Step 3: Run to verify it fails.**
Run: `cargo test -p terminal_share protocol`
Expected: FAIL (no `Control`/`Role`).
- [ ] **Step 4: Implement `protocol.rs`.**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role { Host, Viewer }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Control {
    Hello { role: Role, #[serde(skip_serializing_if = "Option::is_none")] secret: Option<String> },
    Resize { cols: u16, rows: u16 },
    Viewers { n: u32 },
    Bye,
}
```
- [ ] **Step 5: Run to verify it passes.**
Run: `cargo test -p terminal_share protocol`
Expected: PASS (2 tests).
- [ ] **Step 6: Commit.**
```bash
git add crates/terminal_share/Cargo.toml crates/terminal_share/src/protocol.rs crates/terminal_share/src/lib.rs
git commit -m "feat(terminal_share): wire protocol types"
```

### Task 3.2: PtySource trait + standalone shell source

**Files:**
- Create: `crates/terminal_share/src/pty_source.rs`, `crates/terminal_share/src/standalone.rs`
- Modify: `crates/terminal_share/src/lib.rs` (module decls)

- [ ] **Step 1: Define the abstraction** (`pty_source.rs`). The host side depends only on this; Phase 4 supplies a fork-backed impl.
```rust
use tokio::sync::mpsc;

/// A live PTY the relay can mirror: output to subscribe to, input + resize to apply.
pub trait PtySource: Send + 'static {
    /// Receiver of raw output bytes read from the PTY.
    fn take_output(&mut self) -> mpsc::Receiver<Vec<u8>>;
    /// Write raw input bytes to the PTY.
    fn write_input(&self, bytes: Vec<u8>);
    /// Resize the PTY.
    fn resize(&self, cols: u16, rows: u16);
}
```
- [ ] **Step 2: Write the failing test** (`standalone.rs`): spawn a shell, write `echo HELLO_PTY\n`, assert output contains `HELLO_PTY`.
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty_source::PtySource;
    #[tokio::test]
    async fn shell_echoes() {
        let mut src = StandaloneShell::spawn("/bin/bash").unwrap();
        let mut out = src.take_output();
        src.write_input(b"echo HELLO_PTY\n".to_vec());
        let mut buf = String::new();
        while let Ok(Some(chunk)) = tokio::time::timeout(std::time::Duration::from_secs(5), out.recv()).await {
            buf.push_str(&String::from_utf8_lossy(&chunk));
            if buf.contains("HELLO_PTY") { return; }
        }
        panic!("never saw echo; got: {buf}");
    }
}
```
- [ ] **Step 3: Run to verify it fails.**
Run: `cargo test -p terminal_share standalone`
Expected: FAIL (no `StandaloneShell`).
- [ ] **Step 4: Implement `StandaloneShell`** using `portable-pty`: open a PTY, spawn the shell, spawn a blocking reader thread that forwards reads into an mpsc sender, keep the writer + master for `write_input`/`resize`.
```rust
use crate::pty_source::PtySource;
use portable_pty::{native_pty_system, CommandBuilder, PtySize, MasterPty};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct StandaloneShell {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    out: Option<mpsc::Receiver<Vec<u8>>>,
}

impl StandaloneShell {
    pub fn spawn(shell: &str) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })?;
        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        let _child = pair.slave.spawn_command(cmd)?;
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>(256);
        std::thread::spawn(move || {
            let mut b = [0u8; 8192];
            loop {
                match reader.read(&mut b) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => { if tx.blocking_send(b[..n].to_vec()).is_err() { break; } }
                }
            }
        });
        Ok(Self {
            master: Arc::new(Mutex::new(pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            out: Some(rx),
        })
    }
}

impl PtySource for StandaloneShell {
    fn take_output(&mut self) -> mpsc::Receiver<Vec<u8>> { self.out.take().expect("output taken once") }
    fn write_input(&self, bytes: Vec<u8>) { let _ = self.writer.lock().unwrap().write_all(&bytes); }
    fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.lock().unwrap().resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
    }
}
```
- [ ] **Step 5: Run to verify it passes.**
Run: `cargo test -p terminal_share standalone`
Expected: PASS (1 test).
- [ ] **Step 6: Commit.**
```bash
git add crates/terminal_share/src/pty_source.rs crates/terminal_share/src/standalone.rs crates/terminal_share/src/lib.rs
git commit -m "feat(terminal_share): PtySource trait + standalone shell source"
```

### Task 3.3: Relay client — pump loops + share()/unshare()

**Files:**
- Create: `crates/terminal_share/src/relay_client.rs`
- Modify: `crates/terminal_share/src/lib.rs` (public API)

- [ ] **Step 1: Public API in `lib.rs`.**
```rust
pub mod protocol;
pub mod pty_source;
pub mod relay_client;
pub mod standalone;

use pty_source::PtySource;

pub struct ShareConfig {
    pub relay_host: String, // e.g. "term-share.you.workers.dev" or "127.0.0.1:8787"
    pub secret: String,
    pub tls: bool,          // false for local wrangler dev
}

pub struct ShareHandle {
    pub url: String,             // viewer URL to hand out
    cancel: tokio::sync::watch::Sender<bool>,
}

impl ShareHandle {
    pub fn url(&self) -> &str { &self.url }
    pub fn unshare(self) { let _ = self.cancel.send(true); } // drop sockets -> URL dies
}

/// Start sharing `source`. Returns immediately with the viewer URL; pumping runs in the background.
pub fn share(source: impl PtySource, cfg: ShareConfig) -> ShareHandle {
    relay_client::start(source, cfg)
}
```
- [ ] **Step 2: Write the failing end-to-end test** (`tests/end_to_end.rs`). Requires `wrangler dev` on 8787 (skips if not reachable). Shares a standalone shell, connects a fake viewer WS, types via the viewer, asserts shell output returns to the viewer.
```rust
use terminal_share::{share, ShareConfig};
use terminal_share::standalone::StandaloneShell;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn browser_can_drive_shared_shell() {
    // skip if dev relay not up
    if connect_async("ws://127.0.0.1:8787/parties/term/ping?role=viewer").await.is_err() { eprintln!("SKIP: no dev relay"); return; }

    let shell = StandaloneShell::spawn("/bin/bash").unwrap();
    let handle = share(shell, ShareConfig { relay_host: "127.0.0.1:8787".into(), secret: "test-secret".into(), tls: false });
    let room = handle.url().rsplit('/').next().unwrap().to_string();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let (mut v, _) = connect_async(format!("ws://127.0.0.1:8787/parties/term/{room}?role=viewer")).await.unwrap();
    v.send(Message::Text("{\"t\":\"hello\",\"role\":\"viewer\"}".into())).await.unwrap();
    v.send(Message::Binary(b"echo HELLO_BROWSER\n".to_vec())).await.unwrap();

    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) = tokio::time::timeout(std::time::Duration::from_secs(2), v.next()).await {
            if let Message::Binary(b) = msg { seen.push_str(&String::from_utf8_lossy(&b)); if seen.contains("HELLO_BROWSER") { handle.unshare(); return; } }
        }
    }
    panic!("viewer never saw shell output; got: {seen}");
}
```
- [ ] **Step 3: Run to verify it fails.**
Run: `cd relay && (npx wrangler dev --port 8787 --local &) ; sleep 8 ; cd .. && cargo test -p terminal_share --test end_to_end`
Expected: FAIL (no `relay_client::start`).
- [ ] **Step 4: Implement `relay_client.rs`.** Mint a room id + URL, connect host WS, send `hello`, then run three concurrent loops until cancelled: (a) PTY output → `Message::Binary`; (b) incoming `Message::Binary` → `source.write_input`; (c) incoming `resize` text → `source.resize`. On cancel or socket error, close.
```rust
use crate::{ShareConfig, ShareHandle};
use crate::protocol::{Control, Role};
use crate::pty_source::PtySource;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message};

fn room_id() -> String {
    const A: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut r = rand::thread_rng();
    (0..24).map(|_| A[r.gen_range(0..A.len())] as char).collect()
}

pub fn start(mut source: impl PtySource, cfg: ShareConfig) -> ShareHandle {
    let id = room_id();
    let scheme_ws = if cfg.tls { "wss" } else { "ws" };
    let scheme_http = if cfg.tls { "https" } else { "http" };
    let url = format!("{scheme_http}://{}/s/{id}", cfg.relay_host);
    let ws_url = format!("{scheme_ws}://{}/parties/term/{id}?role=host", cfg.relay_host);
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let mut out = source.take_output();
    let source = std::sync::Arc::new(source);

    tokio::spawn(async move {
        let (ws, _) = match connect_async(&ws_url).await { Ok(x) => x, Err(e) => { eprintln!("relay connect failed: {e}"); return; } };
        let (mut tx, mut rx) = ws.split();
        let hello = serde_json::to_string(&Control::Hello { role: Role::Host, secret: Some(cfg.secret.clone()) }).unwrap();
        if tx.send(Message::Text(hello)).await.is_err() { return; }

        loop {
            tokio::select! {
                _ = cancel_rx.changed() => { let _ = tx.send(Message::Text(serde_json::to_string(&Control::Bye).unwrap())).await; let _ = tx.close().await; break; }
                Some(chunk) = out.recv() => { if tx.send(Message::Binary(chunk)).await.is_err() { break; } }
                msg = rx.next() => match msg {
                    Some(Ok(Message::Binary(b))) => source.write_input(b),
                    Some(Ok(Message::Text(t))) => if let Ok(Control::Resize { cols, rows }) = serde_json::from_str(&t) { source.resize(cols, rows); },
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }
    });

    ShareHandle { url, cancel: cancel_tx }
}
```
> Note: `Arc<impl PtySource>` requires `write_input`/`resize` take `&self` (they do). `take_output` is called before the `Arc` wrap.
- [ ] **Step 5: Run to verify it passes.**
Run: `cargo test -p terminal_share --test end_to_end`
Expected: PASS (the viewer sees `HELLO_BROWSER`). Stop the dev relay afterward.
- [ ] **Step 6: Commit.**
```bash
git add crates/terminal_share/src/relay_client.rs crates/terminal_share/src/lib.rs crates/terminal_share/tests/end_to_end.rs
git commit -m "feat(terminal_share): outbound relay client + share/unshare"
```

**Milestone:** End of Phase 3 = a fully working, tested terminal-share system independent of the fork. Open the printed URL in a browser → live read/write of a real shell, through your own relay.

---

## Phase 4: Fork integration

### Task 4.1: Locate the Share action + URL surfacing sites

**Files:**
- Read: the fork's command/action registry and terminal toolbar/menu (search starting points below)

- [ ] **Step 1: Find the action wiring.** Search the stripped fork for where terminal-pane actions are registered (the removed feature left a hole; find the natural seam):
```bash
cd /home/mcs/Documents/git/warp-upstream
grep -rnE "fn .*action|register.*action|Action::|ContextMenu|command_palette" app/src/terminal | grep -iE "pane|terminal|menu" | head -30
```
- [ ] **Step 2: Find a place to show the URL** (toast/notification/inline banner). Search for the existing notification/toast surface:
```bash
grep -rnE "toast|notification|InlineBanner|notify|copy_to_clipboard|clipboard" app/src | head -20
```
- [ ] **Step 3: Record** the exact file:line for (a) where to add a `ShareTerminal`/`UnshareTerminal` action and (b) how to display + copy the URL. No code yet.

### Task 4.2: Fork-backed PtySource

**Files:**
- Create: `crates/terminal_share/src/fork_source.rs`
- Modify: `app/src/terminal/writeable_pty/pty_controller.rs` (add an output broadcast hook — exact symbols from Task 0.1)
- Modify: `crates/terminal_share/Cargo.toml` (feature `fork` gating `fork_source`)

- [ ] **Step 1: Add an output fan-out hook in `pty_controller`.** At the confirmed output dispatch point (Task 0.1 — **VERIFY**: the event-loop thread near `pty_controller.rs:780` that currently feeds the VT parser), clone the raw byte slice into an optional `tokio::sync::broadcast::Sender<Vec<u8>>` held on the controller. Gate behind `Option` so there is zero cost when not sharing.
```rust
// in the controller struct:
share_out: Option<tokio::sync::broadcast::Sender<Vec<u8>>>,

// where raw PTY output bytes are handled (VERIFY exact site from Task 0.1):
if let Some(tx) = &self.share_out { let _ = tx.send(bytes.to_vec()); }
```
Add accessors: `enable_share_output(&mut self) -> broadcast::Receiver<Vec<u8>>` (creates the sender if absent) and `disable_share_output(&mut self)` (drops it). Input reuses the existing `write_bytes`; resize reuses `resize_pty`.
- [ ] **Step 2: Implement `ForkPtySource`** that adapts the broadcast receiver to the `PtySource` mpsc contract and routes writes/resizes through a handle to the controller (a `WeakModel`/channel to the GPUI model, per the fork's threading rules — **VERIFY** the exact handle type when wiring Task 4.3).
```rust
use crate::pty_source::PtySource;
use tokio::sync::mpsc;

pub struct ForkPtySource {
    out: Option<mpsc::Receiver<Vec<u8>>>,
    input: Box<dyn Fn(Vec<u8>) + Send + Sync>,
    resize: Box<dyn Fn(u16, u16) + Send + Sync>,
}

impl ForkPtySource {
    pub fn new(
        mut broadcast_rx: tokio::sync::broadcast::Receiver<Vec<u8>>,
        input: impl Fn(Vec<u8>) + Send + Sync + 'static,
        resize: impl Fn(u16, u16) + Send + Sync + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel(256);
        tokio::spawn(async move {
            while let Ok(b) = broadcast_rx.recv().await { if tx.send(b).await.is_err() { break; } }
        });
        Self { out: Some(rx), input: Box::new(input), resize: Box::new(resize) }
    }
}

impl PtySource for ForkPtySource {
    fn take_output(&mut self) -> mpsc::Receiver<Vec<u8>> { self.out.take().expect("output taken once") }
    fn write_input(&self, bytes: Vec<u8>) { (self.input)(bytes); }
    fn resize(&self, cols: u16, rows: u16) { (self.resize)(cols, rows); }
}
```
- [ ] **Step 3: Build the crate with the fork feature.**
Run: `cargo build -p terminal_share --features fork`
Expected: compiles. (Full-workspace build deferred to Task 4.3.)
- [ ] **Step 4: Commit.**
```bash
git add crates/terminal_share/src/fork_source.rs crates/terminal_share/Cargo.toml app/src/terminal/writeable_pty/pty_controller.rs
git commit -m "feat(terminal_share): fork PTY source + pty_controller output hook"
```

### Task 4.3: Wire the Share / Unshare action

**Files:**
- Modify: the action site from Task 4.1
- Modify: `app/Cargo.toml` (depend on `terminal_share`)
- Modify: the terminal pane model (hold an `Option<ShareHandle>`)

- [ ] **Step 1: Add the dependency.** In `app/Cargo.toml`: `terminal_share = { path = "../crates/terminal_share", features = ["fork"] }`.
- [ ] **Step 2: Hold share state.** Add `share: Option<terminal_share::ShareHandle>` to the terminal pane/session model.
- [ ] **Step 3: Implement `ShareTerminal`.** On invoke: call `enable_share_output()` on the controller to get the broadcast receiver; build a `ForkPtySource` whose `input` closure calls `write_bytes` and `resize` closure calls `resize_pty` (marshalled onto the model thread per fork conventions — **VERIFY** handle type); call `terminal_share::share(source, cfg)` with the configured relay host/secret; store the handle; show + copy `handle.url()` via the toast surface from Task 4.1.
- [ ] **Step 4: Implement `UnshareTerminal`.** Take the stored handle, call `.unshare()`, call `disable_share_output()`. When the session/pane closes, run the same teardown (so the URL never outlives the terminal).
- [ ] **Step 5: Build the app.**
Run: `cargo build -p app` (or the fork's documented app build target)
Expected: compiles.
- [ ] **Step 6: Manual verification.** Launch the app, open a terminal, invoke Share, copy the URL, open it in a browser, type both ways, run a command, invoke Unshare, confirm the browser shows `[session ended]` and the URL 404s on reconnect. Screenshot for the PR.
- [ ] **Step 7: Commit.**
```bash
git add app/Cargo.toml app/src/terminal/...
git commit -m "feat(terminal): Share/Unshare terminal via self-hosted relay"
```

---

## Phase 5: Hardening & polish

### Task 5.1: Config surface

**Files:**
- Modify: the fork's settings model + `crates/terminal_share/src/lib.rs`

- [ ] **Step 1:** Expose settings: relay host, host secret (stored in the OS keychain/secret store the fork already uses — **VERIFY** which), TLS on/off. Default TLS on; localhost dev override allowed.
- [ ] **Step 2:** Validate at share time; surface a clear error toast if the relay is unreachable or the secret is rejected (close codes 4401/4409 from the relay → human messages).
- [ ] **Step 3: Commit.** `git commit -m "feat(terminal_share): settings for relay host/secret/tls"`

### Task 5.2: Reconnect with backoff

**Files:**
- Modify: `crates/terminal_share/src/relay_client.rs`
- Test: extend `tests/end_to_end.rs`

- [ ] **Step 1: Failing test:** kill+restart the dev relay mid-share; assert the host re-establishes the same room id and a viewer reconnecting sees output again.
- [ ] **Step 2: Implement** an outer reconnect loop around the connect+pump block: on socket error, exponential backoff (e.g. 0.5s→8s cap), keep the same room id so the URL stays valid. Stop on cancel.
- [ ] **Step 3: Run + commit.** `git commit -m "feat(terminal_share): reconnect with backoff, stable room id"`

### Task 5.3: View-only mode + viewer count UI

**Files:**
- Modify: `relay/src/server.ts`, `relay/public/viewer.ts`, the fork share action

- [ ] **Step 1:** Add a per-share `mode=view|edit` (default `edit`). In view mode the relay drops viewer→host binary/resize frames. Carry `mode` on the host `hello` and persist it on the room.
- [ ] **Step 2:** Surface the relay `{"t":"viewers","n":N}` count in the app's share indicator.
- [ ] **Step 3: Test (relay):** a view-mode room ignores viewer input. Run + commit. `git commit -m "feat: view-only share mode + viewer count"`

### Task 5.4: Optional end-to-end encryption (additive)

**Files:**
- Modify: `crates/terminal_share/src/relay_client.rs`, `crates/terminal_share/Cargo.toml`, `relay/public/viewer.ts`
- Unchanged: `relay/src/server.ts` (relay already pipes opaque payloads)

- [ ] **Step 1:** Host generates a 256-bit key, puts it in the viewer URL **fragment** (`/s/<id>#k=<base64url>`); the fragment is never sent to the server.
- [ ] **Step 2:** Host AES-256-GCM-encrypts each outbound binary payload (random nonce prepended); decrypts inbound. Use `aes-gcm` crate on the host.
- [ ] **Step 3:** Viewer reads `location.hash`, imports the key via WebCrypto (`crypto.subtle`), encrypts/decrypts symmetrically.
- [ ] **Step 4: Test:** capture a relay-side frame and assert it does **not** contain a known plaintext marker that the viewer nonetheless decrypts correctly. Run + commit. `git commit -m "feat(terminal_share): optional e2e encryption via URL-fragment key"`

### Task 5.5: Deploy + docs

**Files:**
- Create: `relay/README.md`, `docs/terminal-share.md`

- [ ] **Step 1:** Document one-time relay deploy: `cd relay && npx wrangler deploy` then `npx wrangler secret put HOST_SECRET`; record the resulting `*.workers.dev` host (or a custom domain) for the app setting.
- [ ] **Step 2:** Document the user flow, the plaintext-vs-e2e tradeoff, and the free-tier limits (≈3M requests/mo, hibernation while idle).
- [ ] **Step 3: Commit.** `git commit -m "docs: terminal share relay deploy + usage"`

---

## Self-review checklist (run before execution)

- **Spec coverage:** share→URL (3.3/4.3 ✓), live read+write (1.1/2.2/3.3 ✓), nothing when unshared (1.2 no-host reject + 4.4 teardown ✓), self-owned relay/no cloudflared/no stdout (3.3 holds socket, URL is a value ✓), partyserver only (Phase 1 ✓), Cloudflare utilized (DO relay ✓).
- **Cross-subsystem type names:** `Control`/`Role` (Rust) ↔ JSON `{t,role,secret,cols,rows,n}` (relay/viewer) match field-for-field. `PtySource::{take_output,write_input,resize}` used identically in `standalone.rs`, `relay_client.rs`, `fork_source.rs`.
- **VERIFY markers** (real investigation, not placeholders): partyserver method names (1.1), PTY output tap site (0.1/4.2), fork action+toast sites (4.1), GPUI model-thread marshalling handle (4.2/4.3), keychain API (5.1). Each has a dedicated step that resolves it against live code at build time.

## Known risks / decisions deferred to build time
- partyserver pinned-version API drift → resolved by reading `node_modules/partyserver/README.md` in Task 1.1.
- Output tap may expose post-parse events rather than raw bytes → Task 0.1 chooses the pre-parse clone point; if impossible, fall back to serializing the parser's input.
- GPUI threading: `write_bytes`/`resize_pty` take `&mut ModelContext` → the `ForkPtySource` closures must marshal onto the model thread; exact mechanism (channel vs `WeakModel` + `update`) decided in Task 4.3.
- Free-tier per-message request accounting: heavy interactive use bills WS messages against the ≈3M/mo quota; batch tiny output frames (coalesce within a few ms) if it becomes a concern (optional Task, not required for personal use).
