use enum_dispatch::enum_dispatch;

mod cli;
mod utils;
pub use cli::*;

const BUILD_DIR: &str = ".build";

#[allow(async_fn_in_trait)]
#[enum_dispatch]
pub trait CmdExecutor {
    async fn execute(self) -> anyhow::Result<()>;
}
