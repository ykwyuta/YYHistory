"""Serve dist/ with consistent JavaScript/Wasm MIME types on Windows too."""
import mimetypes
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

mimetypes.add_type("text/javascript", ".js")
mimetypes.add_type("text/javascript", ".mjs")
mimetypes.add_type("application/wasm", ".wasm")
root = Path(__file__).resolve().parents[1] / "dist"
handler = partial(SimpleHTTPRequestHandler, directory=str(root))

if __name__ == "__main__":
    print("時空事件簿 preview: http://localhost:8765/", flush=True)
    ThreadingHTTPServer(("127.0.0.1", 8765), handler).serve_forever()
