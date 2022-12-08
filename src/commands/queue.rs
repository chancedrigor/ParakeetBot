/*!
 * Implements the `/queue` command.
 *
 * The bot responds with an embed displaying all the songs in the queue.
 */

use log::instrument;

use crate::{bot, log, Context, Result};

/// Show what's coming up
#[instrument]
#[poise::command(slash_command, guild_only, guild_cooldown = 2)]
pub async fn queue(ctx: Context<'_>) -> Result<()> {
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("expected songbird initialized");
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| log::eyre!("Not in a guild."))?;
    let call = manager
        .get(guild_id)
        .ok_or_else(|| log::eyre!("I'm not in a voice channel."))?
        .clone();

    let call_lock = call.lock().await;
    let queue: bot::Queue = call_lock.queue().into();

    ctx.send(|b| b.embed(|e| e.description(format!("{queue}"))))
        .await?;

    Ok(())
}
