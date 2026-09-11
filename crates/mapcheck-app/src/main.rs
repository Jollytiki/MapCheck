//! MapCheck as a single self-contained executable.
//!
//! The whole web app — HTML, CSS, JavaScript and the Rust/WASM calculation
//! core — is embedded in this binary. Running it serves those files on a
//! loopback port and opens a browser, so there is nothing to install and
//! nothing to keep alongside the executable.

use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use tiny_http::{Header, Request, Response, Server, StatusCode};

mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

use assets::{Asset, ASSETS, EMBEDDED_WASM};

/// Chosen to be memorable and well clear of anything in common use.
const DEFAULT_PORT: u16 = 8731;

struct Options {
    port: Option<u16>,
    open_browser: bool,
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

    // A handful of threads is ample: the only client is one local browser
    // fetching a few files.
    let mut workers = Vec::new();
    for _ in 0..4 {
        let server = Arc::clone(&server);
        workers.push(thread::spawn(move || {
            for request in server.incoming_requests() {
                handle(request);
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
    -p, --port <PORT>    Port to listen on [default: {DEFAULT_PORT}, or any
                         free port if that one is taken]
        --no-browser     Start the server without opening a browser
    -h, --help           Print this help
    -V, --version        Print the version"
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

fn find_asset(path: &str) -> Option<&'static Asset> {
    let path = match path.split(['?', '#']).next().unwrap_or(path) {
        "" | "/" => "/index.html",
        other => other,
    };
    ASSETS.iter().find(|asset| asset.path == path)
}

fn handle(request: Request) {
    let response = match find_asset(request.url()) {
        Some(asset) => Response::new(
            StatusCode(200),
            vec![
                header("Content-Type", asset.mime),
                // The files only change when the binary does, and a stale
                // cache across rebuilds is far more confusing than a refetch.
                header("Cache-Control", "no-store"),
            ],
            Cursor::new(asset.bytes),
            Some(asset.bytes.len()),
            None,
        ),
        None => Response::new(
            StatusCode(404),
            vec![header("Content-Type", "text/plain; charset=utf-8")],
            Cursor::new(b"Not found".as_slice()),
            Some(9),
            None,
        ),
    };

    let _ = request.respond(response);
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
