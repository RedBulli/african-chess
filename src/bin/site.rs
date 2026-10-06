//! Development tool: compiles the Rust engine to WASM, assembles the deployable
//! static site, and serves it locally. Static files only; no game requests.
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const USAGE: &str = "Usage: site build | site serve [--port PORT] [--no-build]";
const ASSETS: [&str; 6] = [
    "index.html",
    "style.css",
    "icon.svg",
    "app.js",
    "engine-client.js",
    "engine-worker.js",
];
const PACKAGE: [&str; 2] = ["african_chess.js", "african_chess_bg.wasm"];
// Keep in step with the Caddyfile.
const POLICY: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
    worker-src 'self'; style-src 'self'; img-src 'self' data:; frame-ancestors 'none'";
const MAX_LINE: u64 = 8192;
const MAX_HEADERS: usize = 100;
const MAX_BODY: u64 = 1 << 20;

fn site_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/web")
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to)
        .map(drop)
        .map_err(|error| format!("Could not copy {}: {error}", from.display()))
}

fn build_site() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new("wasm-pack")
        .args(["build", "--target", "web", "--release"])
        .args(["--out-dir", "web/pkg", "--out-name", "african_chess"])
        .args(["--", "--locked"])
        .current_dir(root)
        .status()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => {
                "Install wasm-pack first: cargo install wasm-pack --locked".to_owned()
            }
            _ => format!("Could not run wasm-pack: {error}"),
        })?;
    if !status.success() {
        return Err(format!("wasm-pack failed: {status}"));
    }
    let (web, site) = (root.join("web"), site_dir());
    fs::create_dir_all(site.join("pkg"))
        .map_err(|error| format!("Could not create {}: {error}", site.display()))?;
    for name in ASSETS {
        copy(&web.join(name), &site.join(name))?;
    }
    for name in PACKAGE {
        copy(&web.join("pkg").join(name), &site.join("pkg").join(name))?;
    }
    println!("Static site built at {}", site.display());
    Ok(())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

/// The file a request target names, or `None` for anything outside the site.
/// Directories are never listed; they only answer with their `index.html`.
fn resolve(root: &Path, target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next()?.strip_prefix('/')?;
    let relative = Path::new(path);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let file = root.join(relative);
    if path.is_empty() || path.ends_with('/') {
        return Some(file.join("index.html"));
    }
    file.is_file().then_some(file)
}

fn respond(
    mut stream: &TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: {POLICY}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn handle(stream: &TcpStream, root: &Path) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.by_ref().take(MAX_LINE).read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut length = 0u64;
    for _ in 0..MAX_HEADERS {
        let mut header = String::new();
        let read = reader.by_ref().take(MAX_LINE).read_line(&mut header)?;
        if read == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    // Read the body before answering, so that closing does not reset the
    // connection under a client that is still sending.
    std::io::copy(
        &mut reader.by_ref().take(length.min(MAX_BODY)),
        &mut std::io::sink(),
    )?;
    let head_only = method == "HEAD";
    if method != "GET" && !head_only {
        return respond(
            stream,
            "501 Not Implemented",
            "text/plain",
            b"Unsupported method\n",
            false,
        );
    }
    match resolve(root, target).map(|file| (content_type(&file), fs::read(file))) {
        Some((kind, Ok(body))) => respond(stream, "200 OK", kind, &body, head_only),
        _ => respond(
            stream,
            "404 Not Found",
            "text/plain",
            b"Not found\n",
            head_only,
        ),
    }
}

fn serve(listener: TcpListener, root: PathBuf) {
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        std::thread::spawn(move || {
            let _ = handle(&stream, &root);
        });
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or(USAGE)?;
    let (mut port, mut build) = (8787u16, true);
    while let Some(key) = args.next() {
        match key.as_str() {
            "--port" if command == "serve" => {
                let value = args.next().ok_or("Option is missing its value")?;
                port = value.parse().map_err(|_| "port must be 0..65535")?
            }
            "--no-build" if command == "serve" => build = false,
            _ => return Err(format!("Unknown option: {key}\n{USAGE}")),
        }
    }
    match command.as_str() {
        "build" => build_site(),
        "serve" => {
            if build {
                build_site()?
            }
            let site = site_dir();
            if !site.join("index.html").is_file()
                || !site.join("pkg/african_chess_bg.wasm").is_file()
            {
                return Err("Build the browser engine first: cargo run --bin site -- build".into());
            }
            let listener = TcpListener::bind(("127.0.0.1", port))
                .map_err(|error| format!("Could not listen on port {port}: {error}"))?;
            let address = listener.local_addr().map_err(|error| error.to_string())?;
            println!("African Chess is ready: http://{address}");
            println!("Static files only; the engine runs in your browser. Press Ctrl+C to stop.");
            serve(listener, site);
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2)
    }
}

/// The development launcher must only serve static browser assets.
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    struct Site {
        root: PathBuf,
        address: SocketAddr,
    }

    impl Site {
        fn start(name: &str) -> Site {
            let root =
                std::env::temp_dir().join(format!("african-chess-{name}-{}", std::process::id()));
            fs::create_dir_all(root.join("pkg")).unwrap();
            fs::write(root.join("index.html"), "<h1>Static game</h1>").unwrap();
            fs::write(root.join("engine-worker.js"), "export {};").unwrap();
            fs::write(root.join("pkg/african_chess_bg.wasm"), b"\0asm").unwrap();
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let served = root.clone();
            std::thread::spawn(move || serve(listener, served));
            Site { root, address }
        }

        /// Status, lowercased response head, and body of one raw request.
        fn request(&self, method: &str, path: &str, body: &str) -> (u16, String, Vec<u8>) {
            let mut stream = TcpStream::connect(self.address).unwrap();
            write!(
                stream,
                "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).unwrap();
            let split = response
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .unwrap();
            let head = String::from_utf8(response[..split].to_vec())
                .unwrap()
                .to_lowercase();
            let status = head.split(' ').nth(1).unwrap().parse().unwrap();
            (status, head, response[split + 4..].to_vec())
        }
    }

    impl Drop for Site {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn serves_html_worker_and_wasm_with_correct_types() {
        let site = Site::start("types");
        for (path, mime) in [
            ("/", "text/html"),
            ("/engine-worker.js", "text/javascript"),
            ("/pkg/african_chess_bg.wasm", "application/wasm"),
        ] {
            let (status, head, body) = site.request("GET", path, "");
            assert_eq!(status, 200, "{path}");
            assert!(head.contains(&format!("content-type: {mime}")), "{head}");
            assert!(head.contains("'wasm-unsafe-eval'"), "{head}");
            assert!(head.contains("x-content-type-options: nosniff"), "{head}");
            assert!(!body.is_empty(), "{path}");
        }
    }

    #[test]
    fn head_requests_carry_no_body() {
        let site = Site::start("head");
        let (status, head, body) = site.request("HEAD", "/index.html", "");
        assert_eq!(status, 200);
        assert!(head.contains("content-length: 20"), "{head}");
        assert!(body.is_empty());
    }

    #[test]
    fn game_api_is_absent() {
        let site = Site::start("api");
        for action in ["state", "ai"] {
            let (status, _, _) = site.request("POST", &format!("/api/{action}"), "{}");
            assert_eq!(status, 501, "{action}");
        }
    }

    #[test]
    fn no_directory_listing_or_source_files() {
        let site = Site::start("listing");
        for path in [
            "/pkg/",
            "/pkg",
            "/Cargo.toml",
            "/../Cargo.toml",
            "/pkg/../../Cargo.toml",
            "//etc/passwd",
            "/missing.js?query",
        ] {
            let (status, _, _) = site.request("GET", path, "");
            assert_eq!(status, 404, "{path}");
        }
    }
}
