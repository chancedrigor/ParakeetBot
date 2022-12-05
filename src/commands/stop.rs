use log::instrument;
use poise::futures_util::TryFutureExt;

use crate::{log, Context, Error, Result};

/// Stop the bot, delete the queue, and leave the call.
#[instrument]
#[poise::command(slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<()> {
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

    let mut call_lock = call.lock().await;

    // Check if in a voice channel
    if call_lock.current_channel().is_none() {
        return Err(log::eyre!("I'm not in a voice channel."));
    }

    let leave = call_lock.leave().map_err(|e| -> Error { e.into() });
    let reply = ctx.say("Buh bye!").map_err(|e| -> Error { e.into() });
    tokio::try_join!(leave, reply)?;

    // Stop everything
    let queue = call_lock.queue();
    queue.stop();

    Ok(())
}
