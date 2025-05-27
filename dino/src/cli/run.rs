use std::fs;

use clap::Parser;
use dino_server::{JsWorker, Req};

use crate::{CmdExecutor, utils::build_project};

#[derive(Debug, Parser)]
pub struct RunOpts {}

impl CmdExecutor for RunOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let filename = build_project(".")?;
        let content = fs::read_to_string(filename)?;
        let worker = JsWorker::try_new(&content)?;

        let req = Req::builder()
            .method("GET".to_string())
            .url("https://www.baidu.com".to_string())
            .build();

        //TODO: normally this should run axum and let it load the worker
        let resp = worker.run("hello", req)?;
        println!("{:?}", resp);
        Ok(())
    }
}
