use crate::bot;
use crate::error::Error;
use crate::youtube;
use crate::Result;
use async_trait::async_trait;
use serenity::builder::CreateApplicationCommandOption;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;
use serenity::model::interactions::application_command::ApplicationCommandOptionType;
use serenity::model::interactions::InteractionResponseType;
use songbird::tracks::Track;
use songbird::tracks::TrackHandle;
use songbird::ytdl;
use tracing::instrument;
use url::Url;

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
                .name("query")
                .description("link to a file, youtube video, or a search query")
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
            None => return Err(Error::NotInGuild.into()),
        };

        //Join the user and get the channel if valid, otherwise send error msg
        let user_id = command.user.id;
        let call = bot::join_user(ctx, guild_id, user_id).await?;

        // Acknowledge the command and update it later (to avoid timeouts)
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        // Parse argument and try to play the audio clip
        if let ApplicationCommandInteractionDataOptionValue::String(query) =
            self.get_option(command, "query")?
        {
            // Process query and put result in the queue
            let (track, track_handle) = process_query(query).await?;
            call.lock().await.enqueue(track);

            //Send msg displaying the queued audio clip
            //TODO Improve message
            command
                .edit_original_interaction_response(&ctx.http, |resp| {
                    resp.content(format!(
                        "Added `{}` to the queue.",
                        track_handle
                            .metadata()
                            .title
                            .as_ref()
                            .unwrap_or(&"???".to_string())
                    ))
                })
                .await?;
        }

        Ok(())
    }
}

#[instrument(level = "debug")]
async fn process_query(query: String) -> Result<(Track, TrackHandle)> {
    let input = match query.parse::<Url>() {
        Ok(url) => ytdl(url).await?,
        Err(_) => {
            let results = youtube::search(query, 1).await?;
            //If this panics, something has gone very wrong
            let url: Url = results
                .first()
                .unwrap()
                .source_url
                .as_ref()
                .unwrap()
                .parse()?;
            ytdl(url).await?
        }
    };

    Ok(songbird::create_player(input))
}
