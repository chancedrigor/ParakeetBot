use crate::Result;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use paste::paste;
use serenity::{
    builder::{CreateApplicationCommand, CreateInteractionResponse},
    client::Context,
    http::Http,
    model::{
        guild::PartialGuild,
        interactions::application_command::{ApplicationCommand, ApplicationCommandInteraction},
    },
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tracing::{debug, error, info};
use tracing::{instrument, Instrument};

mod dc;
mod play;
use dc::Dc;
use play::Play;
// macro_rules! commands {
//     [$($module:tt),*] => {
//         $(mod $module;)*

//         macro_rules! commands_const {
//             () => {
//                 paste! {vec![$(Box::new($module::[<$module:camel>])),*] }
//             }
//         }
//     }
// }

// commands![play, dc];

#[enum_dispatch(SlashCommand)]
#[derive(Debug, EnumIter)]
enum Commands {
    Play,
    Dc,
}

pub async fn register_global_commands(http: &Http) -> Result<()> {
    let commands = ApplicationCommand::set_global_application_commands(http, |com| {
        com.set_application_commands(
            Commands::iter()
                .map(|com| com.slash_command_builder())
                .collect(),
        )
    })
    .await?;
    info!(
        "Globally registered the following commands: {:#?}",
        commands
    );
    Ok(())
}

pub async fn register_guild_commands(http: &Http, guild: PartialGuild) -> Result<()> {
    let commands = guild
        .set_application_commands(http, |com| {
            com.set_application_commands(
                Commands::iter()
                    .map(|com| com.slash_command_builder())
                    .collect(),
            )
        })
        .await?;
    info!(
        "For guild: {}, registered the following commands: {:?}",
        guild.id,
        commands.iter().map(|c| &c.name).collect::<Vec<&String>>()
    );
    Ok(())
}

#[instrument(level = "debug", skip(ctx, command), fields(command_name))]
pub async fn handle_command(ctx: Context, command: ApplicationCommandInteraction) {
    async fn try_handle_command(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> Result<()> {
        let command_name = &command.data.name;
        let response = Commands::iter().find(|com| com.name() == command_name);
        debug!("Matched command to {:?}", response);
        match response {
            Some(com) => {
                com.slash_command_handle(&ctx, &command)
                    .in_current_span()
                    .await?
            }
            None => {
                reply_simple_msg(
                    &ctx,
                    &command,
                    format!("Unrecognized command {}", command_name),
                )
                .await?
            }
        };
        Ok(())
    }

    if let Err(handle_error) = try_handle_command(&ctx, &command).await {
        if let Err(send_error) = command
            .create_interaction_response(&ctx.http, |resp| {
                resp
            .kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| {
                    data
                    .content(format!("{}", handle_error))
                    .allowed_mentions(|mentions| {
                        mentions.replied_user(true)
                    })
                })
            })
            .await
        {
            error!("{}", send_error);
        }
    }
}

#[async_trait]
#[enum_dispatch]
pub trait SlashCommand {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn slash_command_builder(&self) -> CreateApplicationCommand {
        CreateApplicationCommand::default()
            .name(self.name())
            .description(self.description())
            .to_owned()
    }
    async fn slash_command_handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> Result<()> {
        command.create_interaction_response(&ctx.http, |resp| {
            resp.kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|data| {
                data.content(format!("Unimplemented command: {}", self.name()))
            })
        }).await?;
        Ok(())
    }
}

pub async fn reply_simple_msg(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    msg: String,
) -> Result<()> {
    command
        .create_interaction_response(&ctx.http, |resp| {
            resp.kind(
                serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource,
            )
            .interaction_response_data(|data| data.content(msg))
        })
        .await?;
    Ok(())
}
