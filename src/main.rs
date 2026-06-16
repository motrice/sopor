mod ical;
mod svoa;
mod templates;

use std::net::SocketAddr;

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    svoa: svoa::Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let state = AppState {
        svoa: svoa::Client::new(),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/autocomplete", get(autocomplete))
        .route("/preview", get(preview))
        .route("/ics", get(ics))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn index() -> Html<&'static str> {
    Html(templates::INDEX_HTML)
}

#[derive(Deserialize)]
struct QueryParam {
    query: String,
}

async fn autocomplete(
    State(state): State<AppState>,
    Query(q): Query<QueryParam>,
) -> Response {
    if q.query.trim().len() < 2 {
        return Json(Vec::<svoa::Suggestion>::new()).into_response();
    }
    match state.svoa.autocomplete(&q.query).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => {
            tracing::warn!("autocomplete failed: {e}");
            (StatusCode::BAD_GATEWAY, "upstream error").into_response()
        }
    }
}

#[derive(Deserialize)]
struct AddressParam {
    address: String,
}

async fn preview(
    State(state): State<AppState>,
    Query(p): Query<AddressParam>,
) -> Response {
    match state.svoa.search(&p.address).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => {
            tracing::warn!("preview failed: {e}");
            (StatusCode::BAD_GATEWAY, "upstream error").into_response()
        }
    }
}

async fn ics(State(state): State<AppState>, Query(p): Query<AddressParam>) -> Response {
    let schedule = match state.svoa.search(&p.address).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("ics fetch failed: {e}");
            return (StatusCode::BAD_GATEWAY, "upstream error").into_response();
        }
    };

    let body = ical::build_calendar(&p.address, &schedule);
    (
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (
                header::CACHE_CONTROL,
                "public, max-age=3600, stale-while-revalidate=86400",
            ),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"sophamtning.ics\"",
            ),
        ],
        body,
    )
        .into_response()
}
