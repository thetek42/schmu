use std::io::Cursor;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, Result};
use ascii::AsciiString;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const ADDRESS: &str = "0.0.0.0:6969";
const CONNECTION_COUNT: usize = 64;

pub fn start() -> JoinHandle<()> {
    thread::spawn(|| {
        while let Err(e) = run_server() {
            log::error!("webserver failed: {e}");
            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn run_server() -> Result<()> {
    log::info!("starting webserver on {ADDRESS}");
    let server = Server::http(ADDRESS).map_err(|e| anyhow!(e))?;
    let server = Arc::new(server);

    let mut handles = Vec::new();

    for _ in 0..CONNECTION_COUNT {
        let server = Arc::clone(&server);
        handles.push(thread::spawn(move || {
            for request in server.incoming_requests() {
                if let Err(e) = handle_request(request) {
                    log::error!("failed to handle request: {e}");
                }
            }
        }));
    }

    for handle in handles {
        _ = handle.join();
    }

    Ok(())
}

const NOT_FOUND: &str = include_str!("pages/404.html");

fn handle_request(request: Request) -> Result<()> {
    if !matches!(request.method(), Method::Get) {
        request.respond(Response::empty(405))?;
        return Ok(());
    }

    request.respond(Response::not_found())?;
    Ok(())
}

trait ResponseExt: Sized {
    fn html(s: &str) -> Self;
    fn not_found() -> Self;
}

impl ResponseExt for Response<Cursor<Vec<u8>>> {
    fn html(s: &str) -> Self {
        Self::from_string(s).with_header(Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
    }

    fn not_found() -> Self {
        Self::html(NOT_FOUND).with_status_code(StatusCode(404))
    }
}
