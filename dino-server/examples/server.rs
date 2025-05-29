use anyhow::Result;
use dino_server::{ProjectConfig, SwappableAppRouter, TenentRouter, start_server};
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

    let code = r#"
    (function(){
        async function hello(req){
            print(`user id: ${req.params.id}`);
            return {
                status:200,
                headers:{
                    "content-type":"application/json"
                },
                body: JSON.stringify(req),
            };
        }
        return{hello:hello};
    })();
    "#;
    let router = SwappableAppRouter::try_new(code, config.routes)?;

    start_server(
        8888,
        vec![TenentRouter::new("localhost".to_string(), router)],
    )
    .await?;

    Ok(())
}
