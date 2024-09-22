//! * Parakeet-bot is a simple Discord bot meant mostly for single-server use.
#![warn(nonstandard_style)]
#![warn(clippy::missing_docs_in_private_items)]
#![allow(special_module_name)]

mod commands;
mod data;
mod error;
mod lib;
mod log;
mod setup;

use data::Data;
use tracing::instrument;

/// --- Re-exports
pub use error::ParakeetError;
pub use poise::serenity_prelude as serenity;
pub use setup::Config;

/// Type alias for the only [`Context`](poise::Context) type used in this bot.
pub type Context<'a> = poise::Context<'a, Data, ParakeetError>;

#[tokio::main]
#[instrument]
async fn main() -> Result<(), ParakeetError> {
    // Read config file.
    let config = Config::read()?;
    // Initialize logging.
    let _tracing_guard = log::install_tracing(&config);

    let mut client = setup::client(config).await?;
    client.start().await?;

    Ok(())
}
