use crate::bot;
use crate::Result;
use async_trait::async_trait;
use color_eyre::Help;
use serenity::builder::CreateApplicationCommandOption;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;
use serenity::model::interactions::application_command::ApplicationCommandOptionType;
use serenity::model::interactions::InteractionResponseType;
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

    fn options(&self) -> Vec<CreateApplicationCommandOption> {
        let url_opt = {
            CreateApplicationCommandOption::default()
                .name("url")
                .description("a link to a youtube video or an audio/video file")
                .kind(ApplicationCommandOptionType::String)
                .required(true)
                .to_owned()
        };
        vec![url_opt]
    }

    #[instrument(level = "debug", skip(ctx, command), fields(command_name))]
    async fn slash_command_handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> Result<()> {
        //Check if command came from a guild
        let guild_id = match command.guild_id {
            Some(id) => id,
            //Matches the case where the command is used from a direct message
            None => {
                super::reply_simple_msg(ctx, command, "Not in a server!".to_string()).await?;
                return Ok(());
            }
        };

        //Join the user and get the channel if valid, otherwise send error msg
        let user_id = command.user.id;
        let call = bot::join_user(ctx, guild_id, user_id).await?;
        let current_channel = call
            .lock()
            .await
            .current_channel()
            .expect("Not in a channel after joining one");

        //Parse argument and try to play the audio clip
        if let ApplicationCommandInteractionDataOptionValue::String(url) =
            self.get_option(command, "url")?
        {
            let track = songbird::ytdl(&url).await.note(format!("url={}", &url))?;
            call.lock().await.play_source(track);
        }

        //Send msg displaying the queued audio clip
        //TODO Change this message
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|data| {
                        data.content(format!("Joined <#{}>", current_channel))
                    })
            })
            .await?;

        Ok(())
    }
}
