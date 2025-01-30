use std::env;

use anyhow::Result;
use axum::extract::{Path, Query, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{any, get};
use axum::{Form, Json, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::connections;
use crate::socket;
use crate::ytapi;

pub async fn start() -> Result<()> {
    let app = Router::new()
        .route("/submit/{id}", get(get_submit).post(post_submit))
        .route("/ytapi/search", get(ytapi_search))
        .route("/ws", any(websocket))
        .fallback(not_found);

    let port = match env::var("SCHMU_SERVER_PORT") {
        Ok(port) => port.parse().unwrap(),
        Err(_) => shared::consts::SERVER_PORT_SERVER,
    };

    let address = format!("0.0.0.0:{port}");
    log::info!("starting webserver on {address}");
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

const HTML_NOT_FOUND: &str = include_str!("pages/404.html");
const HTML_SUBMIT: &str = include_str!("pages/submit.html");
const HTML_SUCCESS: &str = include_str!("pages/success.html");

async fn get_submit(Path(id): Path<String>) -> impl IntoResponse {
    log::info!("get /submit/{id}");
    if !connections::get().await.exists(&id) {
        return (StatusCode::BAD_REQUEST, "Invalid session").into_response();
    }
    Html(HTML_SUBMIT).into_response()
}

async fn post_submit(
    Path(id): Path<String>,
    Form(form): Form<SubmitPostForm>,
) -> impl IntoResponse {
    log::info!("post /submit/{id}?id={}", form.id);
    if form.id.len() != 11 {
        return (StatusCode::BAD_REQUEST, "Invalid song ID").into_response();
    }
    connections::get().await.submit(&id, &form.id).await;
    Html(HTML_SUCCESS).into_response()
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html(HTML_NOT_FOUND))
}

async fn ytapi_search(Query(query): Query<YtapiSearchQuery>) -> impl IntoResponse {
    log::info!("post /ytapi/search?query={}", query.query);
    match ytapi::search(&query.query).await {
        Ok(songs) => Json(songs).into_response(),
        Err(e) => {
            log::warn!("failed to search on youtube: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(())).into_response()
        }
    }
}

async fn websocket(ws: WebSocketUpgrade) -> impl IntoResponse {
    log::info!("websocket /ws");
    ws.on_upgrade(socket::handle)
}

#[derive(Deserialize)]
struct SubmitPostForm {
    id: String,
}

#[derive(Deserialize)]
struct YtapiSearchQuery {
    query: String,
}
