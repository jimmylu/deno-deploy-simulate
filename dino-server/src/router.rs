use anyhow::Result;
use arc_swap::ArcSwap;
use axum::http::Method;
use matchit::{Match, Router};
use std::{ops::Deref, sync::Arc};

use crate::{ProjectRoutes, error::AppError};

#[derive(Clone)]
pub struct SwappableAppRouter {
    pub routers: Arc<ArcSwap<AppRouterInner>>,
}

pub struct AppRouterInner {
    pub code: String,
    pub router: Router<MethodRoute>,
}

#[derive(Clone)]
pub struct AppRouter(Arc<AppRouterInner>);

#[derive(Debug, Default, Clone)]
pub struct MethodRoute {
    get: Option<String>, // handler name  in js code
    post: Option<String>,
    put: Option<String>,
    delete: Option<String>,
    patch: Option<String>,
    options: Option<String>,
    head: Option<String>,
    connect: Option<String>,
    trace: Option<String>,
}

impl SwappableAppRouter {
    pub fn try_new(code: impl Into<String>, routes: ProjectRoutes) -> Result<Self> {
        let router = Self::get_router(routes)?;
        let inner = AppRouterInner::new(code, router)?;
        Ok(Self {
            routers: Arc::new(ArcSwap::from_pointee(inner)),
        })
    }

    fn get_router(routes: ProjectRoutes) -> Result<Router<MethodRoute>> {
        let mut router = Router::new();
        for (path, methods) in routes {
            let mut method_route = MethodRoute::default();
            for method in methods {
                match method.method {
                    Method::GET => method_route.get = Some(method.handler),
                    Method::POST => method_route.post = Some(method.handler),
                    Method::PUT => method_route.put = Some(method.handler),
                    Method::DELETE => method_route.delete = Some(method.handler),
                    Method::PATCH => method_route.patch = Some(method.handler),
                    Method::OPTIONS => method_route.options = Some(method.handler),
                    Method::HEAD => method_route.head = Some(method.handler),
                    Method::CONNECT => method_route.connect = Some(method.handler),
                    Method::TRACE => method_route.trace = Some(method.handler),
                    v => unreachable!("unsupported method: {:?}", v),
                }
            }
            router.insert(path, method_route)?;
        }
        Ok(router)
    }

    pub fn load(&self) -> AppRouter {
        AppRouter(self.routers.load_full())
    }

    pub fn swap(&self, code: impl Into<String>, routes: ProjectRoutes) -> Result<()> {
        let router = Self::get_router(routes)?;
        let inner = AppRouterInner::new(code, router)?;
        self.routers.store(Arc::new(inner));
        Ok(())
    }
}

#[allow(elided_named_lifetimes)]
impl AppRouter {
    pub fn match_it<'m, 'p>(
        &'m self,
        method: Method,
        path: &'p str,
    ) -> Result<Match<&str>, AppError>
    where
        'p: 'm,
    {
        println!("match_it: {:?}, {:?}", method, path);
        let Ok(ret) = self.router.at(path) else {
            return Err(AppError::RoutePathNotFound(path.to_string()));
        };
        let s = match method {
            Method::GET => ret.value.get.as_deref(),
            Method::POST => ret.value.post.as_deref(),
            Method::PUT => ret.value.put.as_deref(),
            Method::DELETE => ret.value.delete.as_deref(),
            Method::PATCH => ret.value.patch.as_deref(),
            Method::OPTIONS => ret.value.options.as_deref(),
            Method::HEAD => ret.value.head.as_deref(),
            Method::CONNECT => ret.value.connect.as_deref(),
            Method::TRACE => ret.value.trace.as_deref(),
            v => unreachable!("unsupported method: {:?}", v),
        }
        .ok_or_else(|| AppError::RouteMethodNotAllowed(method))?;

        Ok(Match {
            value: s,
            params: ret.params,
        })
    }
}

impl Deref for AppRouter {
    type Target = AppRouterInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AppRouterInner {
    pub fn new(code: impl Into<String>, router: Router<MethodRoute>) -> Result<Self> {
        Ok(Self {
            code: code.into(),
            router,
        })
    }
}
