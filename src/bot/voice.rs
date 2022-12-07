//! Functionality for joining voice calls and automatically adding event handling.
//!
//! Currently the bot listens for idle (empty channel) every 5 minutes and disconnect events.
//! - On idle, the bot stops and deletes the queues, then disconnects.
//! - On disconnect, the bot stops, deletes queues, and removes all global event handlers.
use std::sync::Arc;

use log::{eyre, instrument};
use poise::{async_trait, serenity_prelude as serenity};
use songbird::{Call, Event, EventContext, EventHandler};
use tokio::sync::Mutex;

use crate::{log, Context, Result};

/// Join the author's voice channel based on the given context and register global songbird events.
#[instrument(skip(ctx), fields(author=%ctx.author(), guild=?ctx.guild_id()))]
pub async fn join_author(ctx: &Context<'_>) -> Result<Arc<Mutex<Call>>> {
    let manager = songbird::get(ctx.serenity_context())
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

    let empty_leaver = StopOnIdle {
        call: call.clone(),
        ctx: ctx.serenity_context().to_owned(),
    };

    let dc_stop = StopOnDisconnect { call: call.clone() };
    let mut call_lock = call.lock().await;
    call_lock.add_global_event(
        Event::Periodic(std::time::Duration::from_secs(5 * 60), None),
        empty_leaver,
    );
    call_lock.add_global_event(Event::Core(songbird::CoreEvent::DriverDisconnect), dc_stop);

    Ok(call.clone())
}

/// Unwrap the given option, logging an unexpected `None` with the given message.
/// Cancels the event listener instead of panic.
macro_rules! unwrap_option_cancel {
    ($opt:expr, $msg:tt) => {
        match $opt {
            Some(i) => i,
            None => {
                log::error!($msg);
                return Some(Event::Cancel);
            }
        }
    };
}

/// Unwrap the given result, logging an unexpected `Err` with the given message.
/// The given `id` can be used in the fstring message to displat the error
/// Cancels the event listener instead of panic.
///
/// # Examples
/// ```
/// fn test() -> Option<Event> {
///     let res: Result<i32, &str> = Err("math hard");
///     let num: i32 = unwrap_result_cancel!(res, e, "Failed with error: {e:#}");
///     None
/// }
/// ```
///
macro_rules! unwrap_result_cancel {
    ($opt:expr, $id:ident, $msg:tt) => {
        match $opt {
            Ok(i) => i,
            Err($id) => {
                log::error!($msg);
                return Some(Event::Cancel);
            }
        }
    };
}

/// On idle (i.e. in an empty call), stop all actions and leave the call.
struct StopOnIdle {
    /// Serenity context, needed to access [Cache and Http](serenity::http::CacheHttp).
    ctx: serenity::Context,
    /// Reference to the call that will be dropped.
    call: Arc<Mutex<Call>>,
}

#[async_trait]
impl EventHandler for StopOnIdle {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        let channel_id: serenity::ChannelId = {
            let opt_chan = self.call.lock().await.current_channel();
            unwrap_option_cancel!(opt_chan, "Expected to be in a channel")
                .0 // songbird::ChannelId -> u64
                .into() // u64 -> serenity::ChannelId
        };

        let channel = unwrap_result_cancel!(
            channel_id.to_channel(&self.ctx).await,
            e,
            "Could not find channel: {e:#?}"
        );

        let guild_channel = unwrap_option_cancel!(channel.guild(), "Expected guild channel");

        let members = unwrap_result_cancel!(
            guild_channel.members(&self.ctx).await, // Should never panic
            e,
            "Could not find channel members: {e:#?}"
        );

        // Whether the channel has non-bot members
        let has_members = members.iter().any(|m| !m.user.bot);

        if has_members {
            None // Do nothing but keep the event listener.
        } else {
            unwrap_result_cancel!(
                self.call.lock().await.leave().await, // Leave the channel
                e,
                "Error leaving channel: {e:#?}"
            );
            Some(Event::Cancel) // Cancel the event listener
        }
    }
}

/// On disconnect, stop and delete queue, and remove all global events.
struct StopOnDisconnect {
    /// Reference to the call that will be dropped.
    call: Arc<Mutex<Call>>,
}

#[async_trait]
impl EventHandler for StopOnDisconnect {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        log::info!("Stopping on disconnect!");
        let mut call_lock = self.call.lock().await;
        call_lock.queue().stop();
        call_lock.remove_all_global_events();
        Some(Event::Cancel)
    }
}
