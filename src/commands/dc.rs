use crate::bot;
use crate::Result;

use super::reply_simple_msg;
use super::SlashCommand;

use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct Dc;

#[async_trait]
impl SlashCommand for Dc {
    fn name(&self) -> &str {
        "dc"
    }

    fn description(&self) -> &str {
        "I didn't want to stay here anyway!"
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
            None => {
                reply_simple_msg(ctx, command, "Not in a server!").await?;
                return Ok(());
            }
        };

        bot::disconnect(&ctx, guild_id).await?;
        reply_simple_msg(&ctx, &command, "Buh bye!").await?;

        Ok(())
    }
}
