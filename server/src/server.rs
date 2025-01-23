use anyhow::Result;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Form, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::connections;

const ADDRESS: &str = "0.0.0.0:6969";

pub async fn start() -> Result<()> {
    let app = Router::new()
        .route("/submit/{id}", get(handle_submit_get).post(handle_submit_post))
        .fallback(handle_not_found);

    log::info!("starting webserver on {ADDRESS}");
    let listener = TcpListener::bind(ADDRESS).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

const HTML_NOT_FOUND: &str = include_str!("pages/404.html");
const HTML_SUBMIT: &str = include_str!("pages/submit.html");
const HTML_SUCCESS: &str = include_str!("pages/success.html");

async fn handle_submit_get(Path(id): Path<String>) -> impl IntoResponse {
    log::info!("get /submit/{id}");
    if !connections::get().await.exists(&id) {
        return (StatusCode::BAD_REQUEST, "Invalid session").into_response();
    }
    Html(HTML_SUBMIT).into_response()
}

#[derive(Deserialize)]
struct SubmitPostForm {
    id: String,
}

async fn handle_submit_post(
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

async fn handle_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html(HTML_NOT_FOUND))
}
