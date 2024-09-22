//! Setup for [poise::Framework]

use crate::commands;
use crate::serenity;
use crate::Config;
use crate::Data;
use crate::ParakeetError;

/// Convenient type alias, only this [poise::Framework] type is used.
type Framework = poise::Framework<Data, ParakeetError>;

/// Construct a [poise::Framework]
pub(super) fn framework(config: Config) -> Framework {
    poise::Framework::builder()
        .options(framework_options())
        .setup(|ctx, rdy, fw| framework_setup(ctx, rdy, fw, config))
        .build()
}

/// Configure options for the [Framework]
fn framework_options() -> poise::FrameworkOptions<Data, ParakeetError> {
    poise::FrameworkOptions {
        // Add commands to the framework
        commands: crate::commands::list(),
        // Handle framework errors
        on_error: |e| crate::log::handle_framework_error(e),
        // Log when commands start
        pre_command: |ctx| {
            Box::pin(async move {
                let cmd_name = &ctx.command().name;
                let user = &ctx.author();
                tracing::info!("Started '{cmd_name}' command from {user}.")
            })
        },
        // Log when finishing commands
        post_command: |ctx| {
            Box::pin(async move {
                let cmd_name = &ctx.command().name;
                let user = &ctx.author();
                tracing::info!("Finished '{cmd_name}' command from {user}.")
            })
        },
        ..Default::default()
    }
}

/// Construct future that runs on startup
fn framework_setup<'a>(
    ctx: &'a serenity::Context,
    rdy: &'a serenity::Ready,
    fw: &'a Framework,
    config: Config,
) -> poise::BoxFuture<'a, Result<Data, ParakeetError>> {
    Box::pin(async move {
        // Register the commands
        let commands = &commands::list();
        let app_commands = poise::builtins::create_application_commands(commands);

        serenity::Command::set_global_commands(&ctx, app_commands.clone()).await?;
        if let Some(dev_guild) = config.dev_guild() {
            // This is faster than global registers, useful for development.
            tracing::info!("Registering commands on dev guild.");
            dev_guild.set_commands(ctx, app_commands).await?;
        }

        // Simple message that logs when the bot has initialized
        let bot_name = &rdy.user.name;
        tracing::info!("{bot_name} is ready!");

        let notify_list = config.notify_list(fw);

        let data = Data {
            notify_list,
            ..Default::default()
        };

        Ok(data)
    })
}
