# Browser-only play

Rules, legal moves, and the learned opponent run as Rust WebAssembly in a module
Web Worker. The site consists entirely of static files and makes no game API
requests. Saved games are stored locally in the browser.

## Build and serve

Build prerequisites: the pinned Rust toolchain, Python 3.10+, and wasm-pack.
Install the additional build tools once:

```sh
cargo install wasm-pack --locked
rustup target add wasm32-unknown-unknown
```

From the repository root:

```sh
python3 web/build.py
python3 -m http.server 8787 --bind 127.0.0.1 --directory dist/web
```

Open http://127.0.0.1:8787. Alternatively, `python3 web/server.py` builds and
launches a static development server. It accepts `--port 8788` and `--no-build`.
It never executes game requests or starts a native engine process.

Upload the contents of `dist/web/` to a static HTTP(S) host. Relative module and
asset URLs support deployment under a subdirectory, such as `/african-chess/`.
Serve `.wasm` as `application/wasm` and `.js` as `text/javascript`. If your host
sets a Content Security Policy, allow same-origin scripts, workers, and fetches,
and add `'wasm-unsafe-eval'` to `script-src`. The development launcher supplies
these headers. No cross-origin isolation or special thread headers are needed.

Use HTTP(S); browser module loading does not support opening this site through
`file://`. Once the worker is loaded, gameplay needs no network. There is no
service worker or offline-reload cache. Cancelling an active search replaces the
worker, which requires its assets to remain available from the host or browser
cache.

`web/pkg/` and `dist/` are generated and ignored by Git. Rust release optimization
is enabled; wasm-pack's optional wasm-opt stage is disabled because older
wasm-pack distributions bundle an optimizer incompatible with the pinned Rust
compiler's output. The build uses the committed Cargo lockfile.

## Engine boundary

`src/browser.rs` validates requests and reuses `play::Session`. The WASM export
`game_request(action, payloadJson)` returns JSON or throws an error string.
Actions are `state` and `ai`; payloads contain `moves`, `human`, `level`, and an
optional starting `fen`. Requests are limited to 65536 bytes, 2000 moves, and a
200-character FEN. The native `web-engine` executable is retained for parity tests.

`engine-client.js` manages a single worker and Promise-based requests.
`engine-worker.js` initializes WASM and executes the Rust export. Abort, reset,
undo, timeout, and failures terminate active workers; the next request creates a
new one. Idle workers are reused. Request IDs and the UI generation protect
against stale results. A request has a 20-second timeout. Loading failures show
an error and the existing Try again control retries with a fresh worker.

The opponent embeds the bundled 30-coefficient `models/browser.weights` at build
time; no training tools or run files are needed. The three strengths use the engine's
deterministic depth and node limits. Search has no browser-side Rust clock calls;
the main thread enforces timeouts by terminating the worker.

## Verification

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
node --test web/engine-client.test.mjs
python3 -m unittest discover -s web -p 'test_*.py'
python3 web/build.py
cargo build --locked --release --bin web-engine
```

Install the optional browser-test dependency and Chromium (Node.js/npm required):

```sh
npm install --no-save --package-lock=false playwright
npx playwright install chromium
```

With the static server running (`python3 web/server.py --no-build`):

```sh
node web/browser-test.mjs
node web/wasm-test.mjs
```

`PLAYWRIGHT_MODULE` can point to an existing `playwright` or `playwright-core`
installation. `GAME_URL` overrides the default http://127.0.0.1:8787; include a
trailing slash when serving under a subdirectory. For example, serve `dist/`
and set `GAME_URL=http://127.0.0.1:8787/web/` to exercise prefixed hosting.

The browser tests cover actual human/AI moves, save/restore, both sides, undo,
board flipping, terminating active AI on reset and undo, giraffe pins and
stationary captures, freeze/thaw, promotion, checkmate, failed-WASM retry,
offline play after loading, zero API requests, and mobile layout. Screenshots go
to `web/artifacts/`. The WASM tests compare full JSON results with the native
engine for representative positions and all three strengths, then check invalid
requests and recovery using the same real WASM worker.
