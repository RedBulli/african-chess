"""Build and serve the browser-only game using a local static HTTP server."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from build import SITE, build_site


class Handler(SimpleHTTPRequestHandler):
    extensions_map = {**SimpleHTTPRequestHandler.extensions_map,
                      ".js": "text/javascript", ".wasm": "application/wasm"}

    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Security-Policy",
                         "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; "
                         "worker-src 'self'; style-src 'self'; img-src 'self' data:; "
                         "frame-ancestors 'none'")
        super().end_headers()

    def list_directory(self, path):
        self.send_error(404, "Not found")

    def log_message(self, fmt, *args):
        pass


def create_server(directory, port):
    return ThreadingHTTPServer(("127.0.0.1", port),
                               partial(Handler, directory=str(Path(directory).resolve())))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8787)
    parser.add_argument("--no-build", action="store_true", help="Serve the existing dist/web build")
    args = parser.parse_args()
    if not 0 <= args.port <= 65535:
        parser.error("port must be 0..65535")
    if not args.no_build:
        build_site()
    if not (SITE / "index.html").is_file() or not (SITE / "pkg/african_chess_bg.wasm").is_file():
        parser.error("Build the browser engine first: python3 web/build.py")
    server = create_server(SITE, args.port)
    print(f"African Chess is ready: http://127.0.0.1:{server.server_port}", flush=True)
    print("Static files only; the engine runs in your browser. Press Ctrl+C to stop.", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
