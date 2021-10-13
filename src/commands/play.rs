use std::thread::current;

use crate::bot;
use crate::Result;
use async_trait::async_trait;
use serenity::builder::CreateApplicationCommand;
use serenity::builder::CreateInteractionResponse;
use serenity::client::Context;
use serenity::http::CacheHttp;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::interactions::application_command::ApplicationCommandOptionType;
use tracing::instrument;

use super::SlashCommand;

#[derive(Debug, Default)]
pub struct Play;

#[async_trait]
impl SlashCommand for Play {
    fn name(&self) -> &str {
        "play"
    }

    fn description(&self) -> &str {
        "play an audio clip"
    }

    fn slash_command_builder(&self) -> CreateApplicationCommand {
        CreateApplicationCommand::default()
            .name(self.name())
            .description(self.description())
            //TODO Disable command in private dms by enabling perms at server join
            // .default_permission(false)
            .create_option(|opt| {
                opt.kind(ApplicationCommandOptionType::String)
                    .name("url")
                    .description("a link to a youtube video or an audio/video file")
                    .required(true)
            })
            .to_owned()
    }

    #[instrument(level = "debug", skip(ctx, command), fields(command_name))]
    async fn slash_command_handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> Result<()> {
        let guild_id = match command.guild_id {
            Some(id) => id,
            //Matches the case where the command is used from a direct message
            None => {
                super::reply_simple_msg(ctx, command, "Not in a server!".to_string()).await?;
                return Ok(());
            }
        };

        let user_id = command.user.id;
        let call = bot::join(ctx, guild_id, user_id).await?;
        let current_channel = call
            .lock()
            .await
            .current_channel()
            .expect("Not in a channel after joining one");

        command.create_interaction_response(&ctx.http, |resp| {
            resp
                .kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|data| {
                    data.content(format!("Joined <#{}>", current_channel))
                })
        }).await?;

        Ok(())
    }
}
