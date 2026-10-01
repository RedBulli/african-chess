"""The development launcher must only serve static browser assets."""
import tempfile
import threading
import unittest
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from server import create_server


class StaticServerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        (root / "index.html").write_text("<h1>Static game</h1>")
        (root / "engine-worker.js").write_text("export {};")
        (root / "pkg").mkdir()
        (root / "pkg/african_chess_bg.wasm").write_bytes(b"\0asm")
        self.server = create_server(root, 0)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.stop)
        self.origin = f"http://127.0.0.1:{self.server.server_port}"

    def stop(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def test_serves_html_worker_and_wasm_with_correct_types(self):
        for path, mime in [("/", "text/html"), ("/engine-worker.js", "text/javascript"),
                           ("/pkg/african_chess_bg.wasm", "application/wasm")]:
            with urlopen(self.origin + path) as response:
                self.assertEqual(response.status, 200)
                self.assertEqual(response.headers.get_content_type(), mime)
                self.assertIn("'wasm-unsafe-eval'", response.headers["Content-Security-Policy"])
                self.assertTrue(response.read())

    def test_game_api_is_absent(self):
        for action in ("state", "ai"):
            with self.assertRaises(HTTPError) as caught:
                urlopen(Request(self.origin + "/api/" + action, data=b"{}",
                                headers={"Content-Type": "application/json"}))
            self.assertEqual(caught.exception.code, 501)

    def test_no_directory_listing_or_source_files(self):
        for path in ("/pkg/", "/Cargo.toml", "/../Cargo.toml"):
            with self.assertRaises(HTTPError) as caught:
                urlopen(self.origin + path)
            self.assertEqual(caught.exception.code, 404)


if __name__ == "__main__":
    unittest.main()
