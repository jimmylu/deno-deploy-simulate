use std::{fs, path::Path, time::Duration};

use clap::Parser;
use dino_server::{ProjectConfig, SwappableAppRouter, TenentRouter, start_server};
use notify_debouncer_mini::new_debouncer;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    Layer as _, fmt::Layer, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use crate::{CmdExecutor, utils::build_project};

const MONITOR_FS_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Parser)]
pub struct RunOpts {
    #[clap(short, long)]
    pub port: u16,
}

impl CmdExecutor for RunOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let layer = Layer::new().with_filter(LevelFilter::INFO);
        tracing_subscriber::registry().with(layer).init();

        let (config, code) = get_code_and_config()?;

        let router = SwappableAppRouter::try_new(&code, config.routes)?;
        let routers = vec![TenentRouter::new("localhost", router.clone())];

        tokio::spawn(async_watch(Path::new("."), router));

        start_server(self.port, routers).await?;
        Ok(())
    }
}

fn get_code_and_config() -> Result<(ProjectConfig, String), anyhow::Error> {
    let filename = build_project(".")?;
    let config = filename.replace(".mjs", ".yml");
    let config = ProjectConfig::load(config)?;
    let code = fs::read_to_string(filename)?;
    Ok((config, code))
}

async fn async_watch(p: impl AsRef<Path>, router: SwappableAppRouter) -> Result<(), anyhow::Error> {
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let mut debouncer = new_debouncer(MONITOR_FS_INTERVAL, move |res| {
        tx.blocking_send(res).unwrap();
    })?;

    debouncer
        .watcher()
        .watch(p.as_ref(), notify::RecursiveMode::Recursive)?;

    let mut stream = ReceiverStream::new(rx);
    while let Some(ret) = stream.next().await {
        match ret {
            Ok(events) => {
                let mut need_swap = false;
                for event in events {
                    let ext = event.path.extension().unwrap_or_default();
                    if event.path.ends_with("config.yml") || ext == "ts" {
                        info!("File changed: {:?}", event.path.display());
                        need_swap = true;
                        break;
                    }
                }
                if need_swap {
                    let (config, code) = get_code_and_config()?;
                    router.swap(code, config.routes)?;
                }
            }
            Err(e) => {
                tracing::error!("watch error: {:?}", e);
            }
        }
    }

    Ok(())
}
