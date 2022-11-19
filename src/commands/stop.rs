use log::instrument;

use crate::{log, Context, Result};

/// Stop the bot, delete the queue, and leave the call.
#[instrument]
#[poise::command(slash_command, guild_only, guild_cooldown = 2)]
pub async fn stop(ctx: Context<'_>) -> Result<()> {
    let manager = songbird::get(ctx.discord())
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
    let queue = call_lock.queue();
    queue.stop();
    call_lock.leave().await?;
    ctx.say("Buh bye!").await?;
    Ok(())
}
