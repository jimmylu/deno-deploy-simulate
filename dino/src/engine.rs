use std::collections::HashMap;

use anyhow::Result;
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

// impl<'js> IntoJs<'js> for Request {
//     fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
//         let obj = Object::new(ctx.clone())?;
//         obj.set("method", self.method.into_js(ctx)?)?;
//         obj.set("url", self.url.into_js(ctx)?)?;
//         obj.set("headers", self.headers.into_js(ctx)?)?;
//         obj.set("body", self.body.into_js(ctx)?)?;

//         Ok(obj.into())
//     }
// }

// impl<'js> FromJs<'js> for Response {
//     fn from_js(_ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
//         let obj = value.into_object().unwrap();
//         let body: Option<String> = obj.get("body")?;
//         let headers: HashMap<String, String> = obj.get("headers")?;
//         let status: u16 = obj.get("status")?;

//         Ok(Self {
//             body,
//             headers,
//             status,
//         })
//     }
// }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_worker_should_run() -> anyhow::Result<()> {
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
        let resp = worker.run("hello", req)?;
        println!("{:?}", resp);

        Ok(())
    }
}
