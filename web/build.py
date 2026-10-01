"""Compile the Rust engine to WASM and assemble a deployable static site."""
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
SITE = ROOT / "dist/web"
ASSETS = ("index.html", "style.css", "icon.svg", "app.js", "engine-client.js", "engine-worker.js")


def build_site():
    if not shutil.which("wasm-pack"):
        raise SystemExit("Install wasm-pack first: cargo install wasm-pack --locked")
    subprocess.run([
        "wasm-pack", "build", "--target", "web", "--release", "--out-dir", "web/pkg",
        "--out-name", "african_chess", "--", "--locked",
    ], cwd=ROOT, check=True)
    SITE.mkdir(parents=True, exist_ok=True)
    (SITE / "pkg").mkdir(exist_ok=True)
    for name in ASSETS:
        shutil.copy2(WEB / name, SITE / name)
    for name in ("african_chess.js", "african_chess_bg.wasm"):
        shutil.copy2(WEB / "pkg" / name, SITE / "pkg" / name)
    print(f"Static site built at {SITE}")


if __name__ == "__main__":
    build_site()
