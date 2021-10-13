mod bot;
mod commands;
mod config;
mod error;

use anyhow::Result;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        tracing::error!("{}", e);
        std::process::exit(1)
    }
}

async fn try_main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let conf = config::Config::from_env()?;
    let mut client = bot::initialize(conf).await?;

    client.start().await?;

    Ok(())
}
