use std::sync::Arc;

use color_eyre::eyre::eyre;
use tokio::sync::Mutex;
// use poise::serenity_prelude as serenity;
use tracing::instrument;

use crate::{log, Context, Result};

/// Joins the author's voice channel based on the given context.
#[instrument(skip(ctx), fields(author=%ctx.author(), guild=?ctx.guild_id()))]
pub async fn join_author(ctx: &Context<'_>) -> Result<Arc<Mutex<songbird::Call>>> {
    let manager = songbird::get(ctx.discord())
        .await
        .expect("expected songbird manager");

    let author = ctx.author();

    let (guild_id, voice_states) = match ctx.guild() {
        Some(guild) => (guild.id, guild.voice_states),
        None => {
            let err_rep = eyre!("Could not join {author} because there is no guild from context.");
            log::error!("{err_rep:?}");
            return Err(err_rep);
        }
    };

    // If the author is not in a voice channel, return error
    let channel_id = match voice_states.get(&author.id) {
        Some(vs) => match vs.channel_id {
            Some(id) => id,
            None => {
                let err_rep = eyre!("{author} has a voice state but not in a voice channel.");
                log::error!("{err_rep:?}");
                return Err(err_rep);
            }
        },
        None => {
            let err_rep = eyre!("{author} is not a voice channel.");
            log::info!("{err_rep}");
            return Err(err_rep);
        }
    };

    let (call, join_result) = manager.join(guild_id, channel_id).await;
    join_result?;
    Ok(call)
}
