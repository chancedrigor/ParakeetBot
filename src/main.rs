//! * Parakeet-bot is a simple Discord bot meant mostly for single-server use.
#![warn(nonstandard_style)]
#![warn(clippy::missing_docs_in_private_items)]

use poise::serenity_prelude as serenity;
use songbird::SerenityInit;
mod bot;
mod commands;
mod log;

use color_eyre::eyre::{ErrReport as Error, Result};

/// The data kept between shards (which is nothing atm)
#[derive(Debug)]
pub struct Data;

/// Type alias for the only [`Context`](poise::Context) type used in this bot.
/// This is purely for convenience.
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Returns the value of an enviromental variable.
/// Additionally, attaches the variable's name to any errors.
fn var(varname: &str) -> Result<String> {
    use std::env::var;

    use log::WrapErr;
    var(varname).wrap_err_with(|| format!("Env Var: '{varname}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?; // Load before tracing for debug env vars

    // Console output
    log::install_tracing()?;
    color_eyre::install()?;

    // Your Discord bot token
    let token = var("TOKEN")?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::list(), // Add commands to the framework
            on_error: |e| log::log_to_user(e),
            // On receiving commands, log to console
            pre_command: |ctx| {
                Box::pin(async move {
                    let cmd_name = &ctx.command().name;
                    log::info!("Received '{cmd_name}' command.")
                })
            },
            // When done resolving a command, log to console
            post_command: |ctx| {
                Box::pin(async move {
                    let cmd_name = &ctx.command().name;
                    log::info!("Successfully resolved '{cmd_name}' command.")
                })
            },
            ..Default::default()
        })
        .token(token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, rdy, fw| {
            // Runs on startup
            Box::pin(async move {
                // Simple message that logs when the bot has initialized
                let bot_name = &rdy.user.name;
                let discrim = &rdy.user.discriminator;
                log::info!("{bot_name}#{discrim} is ready!");

                // Registers all the commands on discord
                let commands = &fw.options().commands;
                let app_commmands = poise::builtins::create_application_commands(commands);
                serenity::Command::set_global_application_commands(&ctx.http, |b| {
                    *b = app_commmands;
                    b
                })
                .await?;
                Ok(Data)
            })
        })
        // Register songbird as voice manager
        .client_settings(|c| c.register_songbird());

    framework.run().await?;

    Ok(())
}
