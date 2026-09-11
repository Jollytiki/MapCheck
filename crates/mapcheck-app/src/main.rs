//! MapCheck as a single self-contained executable.
//!
//! The whole web app — HTML, CSS, JavaScript and the Rust/WASM calculation
//! core — is embedded in this binary. Running it serves those files on a
//! loopback port and opens a browser, so there is nothing to install and
//! nothing to keep alongside the executable.

use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tiny_http::{Header, Request, Response, Server, StatusCode};

mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

use assets::{Asset, ASSETS, EMBEDDED_WASM};

/// Chosen to be memorable and well clear of anything in common use.
const DEFAULT_PORT: u16 = 8731;

/// How long after the last browser contact to shut down. The page sends a
/// heartbeat every few seconds, so this only elapses once every tab is gone.
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

/// Until the browser first makes contact, wait this long instead — it covers a
/// slow launch, and bounds the wait if no browser ever arrives.
const STARTUP_GRACE: Duration = Duration::from_secs(90);

/// Injected into the page in app-bundle mode so closing the last tab stops the
/// process. Without it a windowless bundle would linger with no way to quit.
const HEARTBEAT_SCRIPT: &str = r#"<script>
(() => {
    const ping = () => fetch("/heartbeat", { cache: "no-store" }).catch(() => {});
    ping();
    setInterval(ping, 4000);
    // Leaving the page stops the pings, and the server exits shortly after.
})();
</script>
"#;

struct Options {
    port: Option<u16>,
    open_browser: bool,
    /// Exit once the browser stops checking in. On by default inside an app
    /// bundle, off by default at a terminal where Ctrl+C is the natural way out.
    quit_when_idle: bool,
}

/// Whether this executable is running from inside a macOS `.app` bundle.
fn in_app_bundle() -> bool {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().contains(".app/Contents/MacOS/"))
        .unwrap_or(false)
}

fn main() {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => return, // --help / --version
        Err(message) => {
            eprintln!("mapcheck: {message}");
            eprintln!("Try `mapcheck --help`.");
            std::process::exit(2);
        }
    };

    let listener = match bind(options.port) {
        Ok(listener) => listener,
        Err(message) => {
            eprintln!("mapcheck: {message}");
            std::process::exit(1);
        }
    };

    let port = listener.local_addr().expect("bound socket").port();
    let url = format!("http://127.0.0.1:{port}/");

    let server = Arc::new(Server::from_listener(listener, None).expect("start server"));

    // Milliseconds since `started` at the last request, for the watchdog.
    let started = Instant::now();
    let last_seen = Arc::new(AtomicU64::new(0));
    let seen_browser = Arc::new(AtomicU64::new(0));

    println!("MapCheck is running at {url}");
    if !EMBEDDED_WASM {
        println!(
            "  note: built without the Rust/WASM core, so calculations use the\n\
             \x20       JavaScript fallback. Run ./scripts/build-wasm.sh, then rebuild."
        );
    }
    println!("Press Ctrl+C to stop.");

    if options.open_browser {
        open_browser(&url);
    }

    if options.quit_when_idle {
        watchdog(
            started,
            Arc::clone(&last_seen),
            Arc::clone(&seen_browser),
            Arc::clone(&server),
        );
    }

    // A handful of threads is ample: the only client is one local browser
    // fetching a few files.
    let mut workers = Vec::new();
    for _ in 0..4 {
        let server = Arc::clone(&server);
        let last_seen = Arc::clone(&last_seen);
        let seen_browser = Arc::clone(&seen_browser);
        let inject = options.quit_when_idle;
        workers.push(thread::spawn(move || {
            for request in server.incoming_requests() {
                last_seen.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                seen_browser.store(1, Ordering::Relaxed);
                handle(request, inject);
            }
        }));
    }
    for worker in workers {
        let _ = worker.join();
    }
}

fn parse_args() -> Result<Option<Options>, String> {
    let mut options = Options {
        port: None,
        open_browser: true,
        quit_when_idle: in_app_bundle(),
    };
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("mapcheck {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--no-browser" => options.open_browser = false,
            "--quit-when-idle" => options.quit_when_idle = true,
            "--stay-running" => options.quit_when_idle = false,
            "-p" | "--port" => {
                let value = args.next().ok_or("--port needs a number")?;
                options.port = Some(value.parse().map_err(|_| format!("bad port: {value}"))?);
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }

    Ok(Some(options))
}

fn print_help() {
    println!(
        "MapCheck — boundary closure and traverse plotting

USAGE:
    mapcheck [OPTIONS]

Serves the app on a local port and opens it in your browser.

OPTIONS:
    -p, --port <PORT>     Port to listen on [default: {DEFAULT_PORT}, or any
                          free port if that one is taken]
        --no-browser      Start the server without opening a browser
        --quit-when-idle  Exit once the browser stops checking in. The default
                          inside a .app bundle; off at a terminal
        --stay-running    Keep running even with no browser attached
    -h, --help            Print this help
    -V, --version         Print the version"
    );
}

/// Bind to loopback only — this serves a local tool, not a network service.
///
/// With no port requested, fall back to any free one rather than failing when
/// the default is already taken (a second copy already running, most likely).
fn bind(requested: Option<u16>) -> Result<TcpListener, String> {
    let local = |port| SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

    if let Some(port) = requested {
        return TcpListener::bind(local(port)).map_err(|e| format!("cannot use port {port}: {e}"));
    }

    TcpListener::bind(local(DEFAULT_PORT))
        .or_else(|_| TcpListener::bind(local(0)))
        .map_err(|e| format!("cannot listen on 127.0.0.1: {e}"))
}

/// Exits the process once the browser has been quiet for [`IDLE_TIMEOUT`].
///
/// This is what lets a windowless app bundle be closed by closing its tab.
/// Before the browser has ever made contact a longer grace period applies, so
/// a slow launch is not mistaken for an abandoned one.
fn watchdog(
    started: Instant,
    last_seen: Arc<AtomicU64>,
    seen_browser: Arc<AtomicU64>,
    server: Arc<Server>,
) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));

        let elapsed = started.elapsed();
        let quiet_for =
            elapsed.saturating_sub(Duration::from_millis(last_seen.load(Ordering::Relaxed)));

        let expired = if seen_browser.load(Ordering::Relaxed) == 0 {
            elapsed > STARTUP_GRACE
        } else {
            quiet_for > IDLE_TIMEOUT
        };

        if expired {
            println!("No browser attached — shutting down.");
            server.unblock();
            // The worker threads may be parked in accept(); leaving is cleaner
            // than waiting for them, and there is no state to flush.
            std::process::exit(0);
        }
    });
}

fn find_asset(path: &str) -> Option<&'static Asset> {
    let path = match path.split(['?', '#']).next().unwrap_or(path) {
        "" | "/" => "/index.html",
        other => other,
    };
    ASSETS.iter().find(|asset| asset.path == path)
}

fn handle(request: Request, inject_heartbeat: bool) {
    // The heartbeat only needs to have arrived; the caller has already noted it.
    if request.url().starts_with("/heartbeat") {
        let _ = request.respond(Response::new(
            StatusCode(204),
            vec![header("Cache-Control", "no-store")],
            Cursor::new(b"".as_slice()),
            Some(0),
            None,
        ));
        return;
    }

    let response = match find_asset(request.url()) {
        Some(asset) if inject_heartbeat && asset.path == "/index.html" => {
            let page = with_heartbeat(asset.bytes);
            let len = page.len();
            Response::new(
                StatusCode(200),
                vec![
                    header("Content-Type", asset.mime),
                    header("Cache-Control", "no-store"),
                ],
                Cursor::new(page),
                Some(len),
                None,
            )
        }
        Some(asset) => Response::new(
            StatusCode(200),
            vec![
                header("Content-Type", asset.mime),
                // The files only change when the binary does, and a stale
                // cache across rebuilds is far more confusing than a refetch.
                header("Cache-Control", "no-store"),
            ],
            Cursor::new(asset.bytes.to_vec()),
            Some(asset.bytes.len()),
            None,
        ),
        None => Response::new(
            StatusCode(404),
            vec![header("Content-Type", "text/plain; charset=utf-8")],
            Cursor::new(b"Not found".to_vec()),
            Some(9),
            None,
        ),
    };

    let _ = request.respond(response);
}

/// Adds the heartbeat script to the page, just before `</body>` so it runs
/// after the app has loaded. Falls back to appending if that tag is missing.
fn with_heartbeat(page: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(page);
    match text.rfind("</body>") {
        Some(at) => {
            let mut out = String::with_capacity(text.len() + HEARTBEAT_SCRIPT.len());
            out.push_str(&text[..at]);
            out.push_str(HEARTBEAT_SCRIPT);
            out.push_str(&text[at..]);
            out.into_bytes()
        }
        None => {
            let mut out = page.to_vec();
            out.extend_from_slice(HEARTBEAT_SCRIPT.as_bytes());
            out
        }
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

fn open_browser(url: &str) {
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };

    if result.is_err() {
        println!("Could not open a browser automatically — open {url} yourself.");
    }
}
