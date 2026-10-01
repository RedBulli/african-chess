# African Chess

A playable chess variant with freezing bishops, jumping elephants, and giraffes
that can capture without moving. Black moves first.

**Play it now at [achess.verkasalo.com](https://achess.verkasalo.com/)** — free,
no account, straight in your browser.

The game runs entirely in your browser. A Rust engine compiled to WebAssembly
handles rules and the computer opponent in a Web Worker. The site needs only
static hosting: no backend, accounts, or game API.

## Build and play

Install Rust through rustup and Python 3.10 or newer. The repository pins its
Rust toolchain in `rust-toolchain.toml`; rustup selects it automatically.
Install the WebAssembly build tools once from the repository root:

```sh
cargo install wasm-pack --locked
rustup target add wasm32-unknown-unknown
```

Start the development server:

```sh
python3 web/server.py
```

Open **http://127.0.0.1:8787**. The launcher builds the site before serving it.
Stop with Ctrl+C. Use `--port 8788` to change ports, or `--no-build` to serve an
existing build.

Select a piece and a highlighted destination. You can choose either side,
change the opponent strength, flip the board, and undo your previous turn.
The game saves automatically in this browser. Frozen pieces have blue markers;
giraffe stationary captures have dashed targets. Promotion offers all four pieces.

The three opponent strengths use deterministic alpha-beta search with depth
limits of 2/3/4 and node limits of 8,000/50,000/180,000. The opponent evaluates
positions using the 30 coefficients bundled in `models/browser.weights`.
The runtime evaluator in `src/learning.rs` and this weights file are required
build inputs. Training and experimental AI tools are outside this repository.

## Deploy

```sh
python3 web/build.py
```

Upload the contents of `dist/web/` to any static HTTP(S) host, including a
GitHub Pages site. Subdirectory hosting is supported. Serve `.wasm` files as
`application/wasm` and JavaScript as `text/javascript`. No Rust or Python runtime
is needed on the host. Open the game over HTTP(S), not `file://`.

Gameplay can continue offline after loading. Offline page reload is not
provided. See [web/README.md](web/README.md) for hosting headers, the worker
interface, and browser integration tests.

## Development checks

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
node --test web/engine-client.test.mjs
python3 -m unittest discover -s web -p 'test_*.py'
python3 web/build.py
```

The repository contains the web assets, development/build scripts, Rust rules
and opponent runtime, bundled weights, and their tests. `web-engine` is a native
reference executable used to check WebAssembly parity. Build output and local
research files are ignored. `.gitignore` explicitly lists the public files;
add new web/build files to that list when extending the project.

## Rules and notation

Standard starting board, **Black first**. FEN uses standard chess letters:
`N/n` = giraffe, `R/r` = elephant. Uppercase pieces are White.

| Piece | Movement and effect |
| --- | --- |
| Pawn, queen, king | Ordinary chess moves, subject to freezing; no en passant. |
| Bishop | Ordinary diagonal movement/capture. Freezes every adjacent enemy, including diagonal neighbors. |
| Giraffe | Knight movement/capture, plus a stationary capture of the first enemy along its forward file. Any intervening piece blocks the stationary capture. |
| Elephant | Jumps to any of the other 24 squares in a surrounding 5×5 area, within the board; captures at destination. |

Freezing is positional. Frozen pieces cannot move, capture, castle, or give
check. Frozen bishops keep their passive aura. A frozen piece can be captured;
removing its last adjacent enemy bishop immediately thaws it.

King safety is checked after each move, including changed freezing effects.
Freezing a king does not itself win. Checkmate and stalemate apply, and kings
are never captured. Pinned, unfrozen pieces still attack squares, as in chess.

Documented defaults for details not separately specified by the inventor:

- Giraffe “forward” means toward the opponent's original home rank; a
  stationary capture takes an entire turn.
- Castling uses ordinary chess paths and rights; both king and elephant must
  be unfrozen. Pawn double steps and promotion apply. En passant is not allowed. Promotion
  offers queen, bishop, giraffe or elephant.
- The game automatically claims a threefold repetition or fifty-move
  draw once present; claims before an intended move are not modeled.
- Bare kings are drawn. Other ordinary insufficient-material shortcuts are
  not applied: freezing can change mating possibilities. More general dead
  positions are not detected and may eventually draw under the repetition or fifty-move rules.

Moves use coordinates: `a7a6`; promotion: `a7a8n`; stationary capture:
`b2b6@` (the giraffe stays on b2 and removes the enemy on b6).

## License

[MIT](LICENSE)
