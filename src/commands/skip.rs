use std::vec;

use super::{reply_simple_msg, SlashCommand};
use crate::bot;
use crate::error::Error;
use crate::Result;

use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct Skip;

#[async_trait]
impl SlashCommand for Skip {
    fn name(&self) -> &str {
        "skip"
    }

    fn description(&self) -> &str {
        "skip the current item in the queue"
    }

    fn options(&self) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        vec![]
    }

    async fn slash_command_handle(
        &self,
        ctx: &serenity::client::Context,
        command: &serenity::model::interactions::application_command::ApplicationCommandInteraction,
    ) -> Result<()> {
        let guild_id = match command.guild_id {
            Some(id) => id,
            None => return Err(Error::NotInGuild.into()),
        };
        let call = bot::get_call(ctx, guild_id).await?.clone();

        let call_guard = call.lock().await;
        let queue = call_guard.queue();
        let current_item = queue.current();

        match current_item {
            Some(track) => {
                let track_name = track.metadata().title.as_ref().unwrap();
                queue.skip()?;
                reply_simple_msg(ctx, command, format!("Skipped `{}`.", track_name)).await?;
                Ok(())
            }
            None => return Err(Error::EmptyQueue.into()),
        }
    }
}
