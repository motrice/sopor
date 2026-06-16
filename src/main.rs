mod ical;
mod providers;
mod templates;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    registry: Arc<providers::Registry>,
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
        registry: Arc::new(providers::Registry::build()),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(|| async { "ok" }))
        .route("/:kommun", get(kommun_page))
        .route("/:kommun/autocomplete", get(autocomplete))
        .route("/:kommun/preview", get(preview))
        .route("/:kommun/ics", get(ics))
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

async fn index(State(state): State<AppState>) -> Html<String> {
    let kommuner: Vec<(&str, &str)> = state
        .registry
        .iter()
        .map(|p| (p.id(), p.name()))
        .collect();
    Html(templates::render_index(&kommuner))
}

async fn kommun_page(
    State(state): State<AppState>,
    Path(kommun): Path<String>,
) -> Response {
    let Some(provider) = state.registry.get(&kommun) else {
        return (StatusCode::NOT_FOUND, "okänd kommun").into_response();
    };
    Html(templates::render_kommun(
        provider.id(),
        provider.name(),
        provider.placeholder(),
        provider.note(),
    ))
    .into_response()
}

#[derive(Deserialize)]
struct QueryParam {
    query: String,
}

async fn autocomplete(
    State(state): State<AppState>,
    Path(kommun): Path<String>,
    Query(q): Query<QueryParam>,
) -> Response {
    let Some(provider) = state.registry.get(&kommun) else {
        return (StatusCode::NOT_FOUND, "okänd kommun").into_response();
    };
    match provider.autocomplete(&q.query).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => {
            tracing::warn!("autocomplete failed for {kommun}: {e}");
            (StatusCode::BAD_GATEWAY, "upstream error").into_response()
        }
    }
}

#[derive(Deserialize)]
struct AddressParam {
    address: String,
}

#[derive(Serialize)]
struct PreviewEntry {
    date: String,
    weekday: String,
}

#[derive(Serialize)]
struct PreviewSeries {
    waste_type: String,
    frequency: String,
    entries: Vec<PreviewEntry>,
}

async fn preview(
    State(state): State<AppState>,
    Path(kommun): Path<String>,
    Query(p): Query<AddressParam>,
) -> Response {
    let Some(provider) = state.registry.get(&kommun) else {
        return (StatusCode::NOT_FOUND, "okänd kommun").into_response();
    };
    match provider.schedule(&p.address).await {
        Ok(s) => {
            let series: Vec<PreviewSeries> = s
                .series
                .into_iter()
                .map(|series| PreviewSeries {
                    waste_type: series.waste_type,
                    frequency: series.frequency_text,
                    entries: series
                        .anchor
                        .into_iter()
                        .map(|d| PreviewEntry {
                            date: d.format("%Y-%m-%d").to_string(),
                            weekday: swedish_weekday(d),
                        })
                        .collect(),
                })
                .collect();
            Json(series).into_response()
        }
        Err(e) => {
            tracing::warn!("preview failed for {kommun}: {e}");
            (StatusCode::BAD_GATEWAY, "upstream error").into_response()
        }
    }
}

async fn ics(
    State(state): State<AppState>,
    Path(kommun): Path<String>,
    Query(p): Query<AddressParam>,
) -> Response {
    let Some(provider) = state.registry.get(&kommun) else {
        return (StatusCode::NOT_FOUND, "okänd kommun").into_response();
    };
    let schedule = match provider.schedule(&p.address).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("ics fetch failed for {kommun}: {e}");
            return (StatusCode::BAD_GATEWAY, "upstream error").into_response();
        }
    };

    let body = ical::build_calendar(provider.id(), &schedule);
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

fn swedish_weekday(date: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    match date.weekday() {
        chrono::Weekday::Mon => "Måndag",
        chrono::Weekday::Tue => "Tisdag",
        chrono::Weekday::Wed => "Onsdag",
        chrono::Weekday::Thu => "Torsdag",
        chrono::Weekday::Fri => "Fredag",
        chrono::Weekday::Sat => "Lördag",
        chrono::Weekday::Sun => "Söndag",
    }
    .to_string()
}
