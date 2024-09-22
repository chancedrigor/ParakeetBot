//! Implements the `/skip` command.
//!
//! The bot will skip the current track and start playing the next one
//! in the queue (if there is one).

use tracing::instrument;

use crate::data::GetData;
use crate::error::UserError;
use crate::lib;
use crate::Context;
use crate::ParakeetError;

/// Skips the current audio track.
#[instrument(skip(ctx))]
#[poise::command(slash_command, guild_only, guild_cooldown = 2)]
pub async fn skip(ctx: Context<'_>) -> Result<(), ParakeetError> {
    let call = lib::call::get_call(&ctx).await?;

    let call = call.lock().await;

    let queue = call.queue();
    match queue.current() {
        None => Err(UserError::EmptyQueue)?,
        Some(handle) => {
            let meta = {
                let guild_data = ctx.guild_data().await?;
                let queue = guild_data.lock().await;
                queue.queue_metadata.clone()
            };
            let current_meta = meta.front().await.ok_or(UserError::EmptyQueue)?;
            let title = current_meta.title.unwrap_or("<MISSING_TITLE>".to_string());
            tracing::info!("Skipping {title}");
            handle.stop()?;
            ctx.reply(format!("Skipping `{title}`")).await?;
        }
    }

    Ok(())
}
