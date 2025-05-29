mod config;
mod engine;
mod error;
mod router;
use std::collections::HashMap;

use anyhow::Result;
use axum::{
    Router,
    body::Bytes,
    extract::{Query, Request, State},
    response::{IntoResponse, Response},
    routing::any,
};
use axum_extra::extract::Host;
use config::ProjectRoute;
use dashmap::DashMap;
use error::AppError;
use indexmap::IndexMap;
use router::AppRouter;
pub use router::SwappableAppRouter;
use tokio::net::TcpListener;
use tracing::info;

pub use config::ProjectConfig;
pub use engine::{JsWorker, Req, Res};
type ProjectRoutes = IndexMap<String, Vec<ProjectRoute>>;

#[derive(Clone)]
pub struct AppState {
    // host -> router
    routers: DashMap<String, SwappableAppRouter>,
}

#[derive(Clone)]
pub struct TenentRouter {
    host: String,
    router: SwappableAppRouter,
}

pub async fn start_server(port: u16, routers: Vec<TenentRouter>) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(addr).await?;

    info!("Server is running on {}", listener.local_addr()?);

    let map = DashMap::new();
    for TenentRouter { host, router } in routers {
        map.insert(host, router);
    }

    let state = AppState::new(map);
    let app = Router::new()
        .route("/{*path}", any(handler))
        .with_state(state);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

// 修复总结
// 我修复了 dino-server/src/lib.rs 中的编译错误。主要问题是 handler 函数的签名不符合 Axum 0.8 的 Handler trait 要求。
// 主要修改：
// 更改了 handler 函数的参数顺序和类型：
// 移除了 parts: Parts 参数
// 移除了 body: Option<Bytes> 参数
// 添加了 request: Request 参数
// 移除了 Host 参数中的 mut 关键字
// 在 handler 函数内部处理请求分解：
// 使用 request.into_parts() 来获取 parts 和 body
// 使用 axum::body::to_bytes() 来异步读取请求体
// 更新了导入语句：
// 添加了 Request 导入
// 移除了未使用的 Method 和 Uri 导入
// 更新了 assemble_req 函数的类型签名：
// 将 Parts 改为 axum::http::request::Parts 以明确类型
// 这些修改使得代码符合 Axum 0.8 的 Handler trait 要求，现在可以正常编译并且没有任何警告。

// we only support JSON requests and return JSON responses
async fn handler(
    State(state): State<AppState>,
    Host(host): Host,
    Query(query): Query<HashMap<String, String>>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    let (parts, body) = request.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX).await.ok();

    let router: AppRouter = get_router_by_host(host, state)?;
    let matched = router.match_it(parts.method.clone(), parts.uri.path())?;
    let handler = matched.value;

    info!(
        "method:{}, path:{}, query:{:?}, body:{:?}",
        parts.method,
        parts.uri.path(),
        query,
        body
    );

    let req = assemble_req(&matched, &parts, query, body)?;

    info!("req: {:?}", req);
    let worker = JsWorker::try_new(&router.code)?;

    let res = worker.run(handler, req)?;

    info!("res: {:?}", res);
    Ok(Response::from(res))
}

fn assemble_req(
    matched: &matchit::Match<'_, '_, &str>,
    parts: &axum::http::request::Parts,
    query: HashMap<String, String>,
    body: Option<Bytes>,
) -> Result<Req, AppError> {
    let params: HashMap<String, String> = matched
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let headers = parts
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_string()))
        .collect();
    let query = query.into_iter().map(|(k, v)| (k, v.to_string())).collect();
    let body = body.and_then(|v| String::from_utf8(v.into()).ok());

    let req = Req::builder()
        .method(parts.method.to_string())
        .url(parts.uri.to_string())
        .headers(headers)
        .query(query)
        .params(params)
        .body(body.unwrap_or_default())
        .build();

    Ok(req)
}

fn get_router_by_host(mut host: String, state: AppState) -> Result<AppRouter, AppError> {
    let _ = host.split_off(host.find(':').unwrap_or(host.len()));
    let router = state
        .routers
        .get(&host)
        .ok_or(AppError::HostNotFound(host))?;
    Ok(router.load())
}

impl AppState {
    pub fn new(routers: DashMap<String, SwappableAppRouter>) -> Self {
        Self { routers }
    }
}

impl TenentRouter {
    pub fn new(host: impl Into<String>, router: SwappableAppRouter) -> Self {
        Self {
            host: host.into(),
            router,
        }
    }
}
