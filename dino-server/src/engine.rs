use std::collections::HashMap;

use anyhow::Result;
use axum::{body::Body, response::Response};
use dino_macro::{FromJs, IntoJs};
use rquickjs::{Context, Function, Object, Promise, Runtime};
use typed_builder::TypedBuilder;

#[allow(unused)]
pub struct JsWorker {
    rt: Runtime,
    ctx: Context,
}

#[derive(Debug, TypedBuilder, IntoJs)]
pub struct Req {
    #[builder(setter(into))]
    pub method: String,
    #[builder(setter(into))]
    pub url: String,
    #[builder(default)]
    pub query: HashMap<String, String>,
    #[builder(default)]
    pub params: HashMap<String, String>,
    #[builder(default)]
    pub headers: HashMap<String, String>,
    #[builder(default, setter(strip_option))]
    pub body: Option<String>,
}

#[allow(unused)]
#[derive(Debug, FromJs)]
pub struct Res {
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub status: u16,
}

fn print(msg: String) {
    println!("{}", msg);
}

impl JsWorker {
    pub fn try_new(module: &str) -> Result<Self> {
        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;

        ctx.with(|ctx| {
            let global = ctx.globals();

            let ret: Object = ctx.eval(module)?;
            global.set("handlers", ret)?;
            global.set(
                "print",
                Function::new(ctx.clone(), print)?.with_name("print")?,
            )?;

            Ok::<_, anyhow::Error>(())
        })?;

        Ok(Self { rt, ctx })
    }

    pub fn run(&self, name: &str, req: Req) -> anyhow::Result<Res> {
        // self.ctx.with(|ctx| {
        //     ctx.eval_promise(code)?.finish::<()>()?;

        //     Ok::<_, anyhow::Error>(())
        // })?;
        self.ctx.with(|ctx| {
            let global = ctx.globals();
            let handlers: Object = global.get("handlers")?;
            let fun: Function = handlers.get(name)?;
            let v: Promise = fun.call((req,))?;

            Ok::<_, anyhow::Error>(v.finish()?)
        })
    }
}

impl From<Res> for Response {
    fn from(res: Res) -> Self {
        let mut builder = Response::builder().status(res.status);
        for (k, v) in res.headers {
            builder = builder.header(k, v);
        }
        if let Some(body) = res.body {
            builder.body(body.into()).unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn js_worker_should_run() -> anyhow::Result<()> {
        // let code = r#"
        // (function(){async function hello(){print("hello world");return"hello";}return{hello:hello};})();
        // "#;
        let code = r#"
           (function(){async function hello(req){print(`request: ${req}`);return{headers:{"content-type":"text/plain"},status:200,body:"hello world"};}return{hello:hello};})();
        "#;
        let worker = JsWorker::try_new(code)?;
        let req = Req::builder()
            .method("GET".to_string())
            .url("https://www.baidu.com".to_string())
            .headers(HashMap::from([(
                "content-type".to_string(),
                "text/plain".to_string(),
            )]))
            .build();
        let resp: Response = worker.run("hello", req)?.into();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");

        // 获取 body 字符串
        let (_parts, body): (_, Body) = resp.into_parts();
        let body_bytes = to_bytes(body, usize::MAX).await?;
        let body_string = String::from_utf8(body_bytes.to_vec())?;
        assert_eq!(body_string, "hello world");

        Ok(())
    }
}
