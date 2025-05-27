mod config;
mod engine;
mod error;
mod router;
use std::collections::HashMap;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Query, Request, State},
    response::IntoResponse,
    routing::any,
};
use axum_extra::extract::Host;
use config::ProjectRoute;
use dashmap::DashMap;
use error::AppError;
use indexmap::IndexMap;
pub use router::SwappableAppRouter;
use serde_json::json;
use tokio::net::TcpListener;
use tracing::info;

pub use config::ProjectConfig;
pub use engine::{JsWorker, Req, Res};
type ProjectRoutes = IndexMap<String, Vec<ProjectRoute>>;

#[derive(Clone)]
pub struct AppState {
    routers: DashMap<String, SwappableAppRouter>,
}

impl AppState {
    pub fn new(routers: DashMap<String, SwappableAppRouter>) -> Self {
        Self { routers }
    }
}

pub async fn start_server(port: u16, routers: DashMap<String, SwappableAppRouter>) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(addr).await?;

    info!("Server is running on {}", listener.local_addr()?);

    let state = AppState::new(routers);
    let app = Router::new()
        .route("/{*path}", any(handler))
        .with_state(state);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

// we only support JSON requests and return JSON responses
async fn handler(
    State(state): State<AppState>,
    Host(mut host): Host,
    Query(query): Query<serde_json::Value>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let _ = host.split_off(host.find(':').unwrap_or(host.len()));
    // Extract body if present
    let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|_| AppError::HostNotFound("Failed to read request body".to_string()))?;

    let body = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes)?
    };

    info!("host: {:?}, path: {:?}, method: {:?}", host, path, method);

    let router = state
        .routers
        .get(&host)
        .ok_or(AppError::HostNotFound(host))?
        .load();

    let matched = router.match_it(method, &path)?;
    let handler = matched.value;
    let params = matched
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<String, String>>();

    Ok(Json(json!({
        "handler": handler,
        "params": params,
        "body": body,
        "query": query,
    })))
}
