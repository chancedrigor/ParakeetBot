mod bot;
mod commands;
mod config;
mod error;
mod youtube;

use color_eyre::Result;
use dotenv::dotenv;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        tracing::error!("{:?}", e);
        std::process::exit(1)
    }
}

async fn try_main() -> Result<()> {
    dotenv().ok();

    install_tracing();
    color_eyre::install()?;

    let conf = config::Config::from_env()?;
    let mut client = bot::initialize(conf).await?;

    client.start().await?;

    Ok(())
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::fmt;
    use tracing_subscriber::EnvFilter;

    let fmt_layer = fmt::layer().with_target(true).pretty();

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init()
}
