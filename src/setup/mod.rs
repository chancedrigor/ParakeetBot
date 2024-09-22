//! Defines and implements custom bot functionality.

mod config;
mod framework;

use songbird::SerenityInit;

use crate::data::HttpKey;
use crate::serenity;
use crate::ParakeetError;

pub use config::Config;

/// Constructs a [serenity::Client] with initialized [songbird] and [reqwest::Client].
pub(super) async fn client(config: Config) -> Result<serenity::Client, ParakeetError> {
    // Get discord token from config file
    let token = config.token()?;

    // Intents we wish to use
    // See https://discord.com/developers/docs/topics/gateway#gateway-intents
    let intents = serenity::GatewayIntents::non_privileged();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework::framework(config))
        .register_songbird()
        .type_map_insert::<HttpKey>(reqwest::Client::new())
        .await?;

    Ok(client)
}
