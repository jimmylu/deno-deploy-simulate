use anyhow::Result;
use axum::http::Method;
use dashmap::DashMap;
use dino_server::{ProjectConfig, SwappableAppRouter, start_server};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = include_str!("../fixtures/config.yml");
    let config: ProjectConfig = serde_yaml::from_str(config)?;

    println!("router: {:?}", config.routes);

    let router = DashMap::new();
    router.insert(
        "localhost".to_string(),
        SwappableAppRouter::try_new(config.routes)?,
    );

    println!(
        "match: {:?}",
        router
            .get("localhost")
            .unwrap()
            .load()
            .match_it(Method::GET, "/api/hello/123")
    );

    start_server(8888, router).await?;
    Ok(())
}
