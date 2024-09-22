//! Manages [voice calls](Call).
//!
//! Currently the bot monitors for the following:
//! - On idle (alone for some time), the bot stops and deletes the queues, then disconnects.
//! - On disconnect, the bot stops, deletes queues, and removes all global event handlers.

use std::sync::Arc;

use songbird::input::Input;
use songbird::tracks::TrackHandle;
use tokio::sync::Mutex;
use tracing::instrument;

use crate::data::TrackMetadata;
use crate::error::UserError;

use crate::data::GetData;
use crate::lib::events;
use crate::Context;
use crate::ParakeetError;

/// Convenience type alias for [songbird::Call].
pub type CallRef = Arc<Mutex<songbird::Call>>;
/// Convenience type alias for [songbird::Songbird].
type Manager = Arc<songbird::Songbird>;

/// Alias for discovery.
/// Must always use this function to initialize a call.
pub use events::init_global_events as get_call;

/// Get the [Manager] from [Context]
pub async fn get_manager(ctx: &Context<'_>) -> Result<Manager, ParakeetError> {
    songbird::get(ctx.serenity_context())
        .await
        .ok_or(ParakeetError::MissingFromSetup {
            reason: "Expecting songbird manager.".to_string(),
        })
}

/// Join the author's voice channel and register global songbird events.
#[instrument(skip(ctx), fields(author=%ctx.author(), guild=?ctx.guild_id(), channel=?ctx.channel_id()))]
pub async fn join_author(ctx: &Context<'_>) -> Result<CallRef, ParakeetError> {
    // Initializes only once
    events::init_global_events(ctx).await?;

    let manager = get_manager(ctx).await?;
    let author = ctx.author();

    // Try to find the user's guild
    let (guild_id, voice_states) = match ctx.guild() {
        Some(guild) => (guild.id, guild.voice_states.clone()),
        None => Err(UserError::NotInGuild)?,
    };

    // Try to find the user's voice channel
    let channel_id = match voice_states.get(&author.id) {
        Some(vs) => match vs.channel_id {
            Some(id) => id,
            None => Err(UserError::NotInVoice)?,
        },
        None => Err(UserError::NotInVoice)?,
    };

    tracing::info!(
        "Joining {user} at {guild}",
        user = author.name,
        guild = guild_id.name(ctx).unwrap_or("<MISSING GUILD>".to_string())
    );

    // Try to join the call.
    let call = manager.join(guild_id, channel_id).await?;

    Ok(call)
}

/// Add [Input] to the back of the queue.
pub async fn enqueue(
    ctx: &Context<'_>,
    call: &CallRef,
    mut input: Input,
) -> Result<TrackHandle, ParakeetError> {
    tracing::debug!("Adding to the queue.");

    let queue_meta = {
        let guild_data = ctx.guild_data().await?;
        let queue = guild_data.lock().await;
        queue.queue_metadata.clone()
    };

    let metadata = TrackMetadata::from_input(&mut input).await?;

    queue_meta.push_back(metadata).await;

    let track_handle = {
        let mut call = call.lock().await;
        call.enqueue_input(input).await
    };

    Ok(track_handle)
}
