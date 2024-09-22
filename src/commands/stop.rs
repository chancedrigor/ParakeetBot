//! Implements the `/stop` command.
//!
//! This stops all bot actions, clears the queue, and disconnects the
//! bot from the current voice channel.

use tracing::instrument;

use crate::lib;
use crate::Context;
use crate::ParakeetError;

/// Stop the bot, delete the queue, and leave the call.
#[instrument]
#[poise::command(slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), ParakeetError> {
    let call = lib::call::get_call(&ctx).await?;
    let mut call = call.lock().await;

    tracing::info!("Stopping the queue.");
    call.queue().stop();
    call.leave().await?;
    ctx.reply("Queue deleted.").await?;
    Ok(())
}
