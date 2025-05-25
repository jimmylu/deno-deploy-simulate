use anyhow::Result;
use clap::Parser;
use dino::CmdExecutor;
use dino::Opts;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.cmd.execute().await?;
    Ok(())
}
