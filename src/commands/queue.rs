//! Implements the `/queue` command.
//!
//! The bot responds with an embed displaying all the songs in the queue.

use poise::CreateReply;
use serenity::CreateEmbed;
use tracing::instrument;

use crate::data::GetData;
use crate::data::TrackMetadata;
use crate::error::UserError;
use crate::serenity;
use crate::Context;
use crate::ParakeetError;

/// Show what's coming up
#[instrument]
#[poise::command(slash_command, guild_only, guild_cooldown = 2)]
pub async fn queue(ctx: Context<'_>) -> Result<(), ParakeetError> {
    let guild = ctx.guild().ok_or(UserError::NotInGuild)?.name.clone();

    let queue_meta = {
        let guild_data = ctx.guild_data().await?;
        let lock = guild_data.lock().await;
        lock.queue_metadata.clone()
    };

    let mut embed = CreateEmbed::default()
        .description(queue_meta.display_string().await)
        .title(format!("{guild} Queue"));

    // Add thumbnail if front has a thumbnail.
    if let Some(TrackMetadata {
        thumbnail_url: Some(url),
        ..
    }) = queue_meta.front().await
    {
        embed = embed.thumbnail(url)
    };

    let reply = CreateReply::default().embed(embed);

    ctx.send(reply).await?;

    Ok(())
}
