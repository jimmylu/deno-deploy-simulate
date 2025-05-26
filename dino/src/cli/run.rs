use std::{collections::HashMap, fs};

use clap::Parser;

use crate::{
    CmdExecutor,
    engine::{JsWorker, Req},
    utils::build_project,
};

#[derive(Debug, Parser)]
pub struct RunOpts {}

impl CmdExecutor for RunOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let filename = build_project(".")?;
        let content = fs::read_to_string(filename)?;
        println!("content: {}", content);
        let worker = JsWorker::try_new(&content)?;

        let req = Req::builder()
            .method("GET".to_string())
            .url("https://www.baidu.com".to_string())
            .headers(HashMap::from([(
                "content-type".to_string(),
                "text/plain".to_string(),
            )]))
            .build();

        //TODO: normally this should run axum and let it load the worker
        let resp = worker.run("hello", req)?;
        println!("resp: {:?}", resp);
        Ok(())
    }
}
