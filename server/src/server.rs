use std::io::{self, Cursor};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Result, anyhow};
use ascii::AsciiString;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::connections;

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

const HTML_NOT_FOUND: &str = include_str!("pages/404.html");
const HTML_SUBMIT: &str = include_str!("pages/submit.html");
const HTML_SUCCESS: &str = include_str!("pages/success.html");

fn handle_request(request: Request) -> io::Result<()> {
    log::info!("{} {}", request.method(), request.url());
    match request.method() {
        Method::Get => match request.url() {
            url if url.starts_with("/submit/") => handle_submit_get(request),
            _ => request.respond(Response::not_found()),
        },
        Method::Post => match request.url() {
            url if url.starts_with("/submit/") => handle_submit_post(request),
            _ => request.respond(Response::not_found()),
        },
        _ => request.respond(Response::empty(405)),
    }
}

fn handle_submit_get(request: Request) -> io::Result<()> {
    let id = request.url()[8..].trim_end_matches('/');
    if !connections::get().exists(id) {
        return request.respond(Response::error("Invalid session"));
    }
    request.respond(Response::html(HTML_SUBMIT))
}

fn handle_submit_post(mut request: Request) -> io::Result<()> {
    let id = {
        let id = request.url()[8..].trim_end_matches('/');
        if !connections::get().exists(id) {
            return request.respond(Response::error("Invalid session"));
        }
        id.to_owned()
    };

    let mut params = String::new();
    request.as_reader().read_to_string(&mut params)?;
    let params = params.split('&');
    for param in params {
        let Some((key, value)) = param.split_once('=') else {
            return request.respond(Response::bad_request());
        };
        if key == "id" && value.len() == 11 {
            connections::get().submit(&id, value);
            return request.respond(Response::html(HTML_SUCCESS));
        }
    }

    request.respond(Response::bad_request())
}

trait ResponseExt: Sized {
    fn html(s: &str) -> Self;
    fn not_found() -> Self;
    fn error(msg: &str) -> Self;
}

impl ResponseExt for Response<Cursor<Vec<u8>>> {
    fn html(s: &str) -> Self {
        Self::from_string(s).with_header(Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
    }

    fn not_found() -> Self {
        Self::html(HTML_NOT_FOUND).with_status_code(404)
    }

    fn error(msg: &str) -> Self {
        Self::html(&format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Schmu - 404</title>
            </head>
            <body>
                <h3>Error</h3>
                <p>{msg}</p>
            </body>
            </html>
            "#
        ))
        .with_status_code(500)
    }
}

trait ResponseExt2: Sized {
    fn bad_request() -> Self;
}

impl ResponseExt2 for Response<io::Empty> {
    fn bad_request() -> Self {
        Self::empty(400)
    }
}
