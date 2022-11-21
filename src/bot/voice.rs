use std::sync::Arc;

use log::eyre;
use poise::{async_trait, serenity_prelude as serenity};
use songbird::{Call, Event, EventContext, EventHandler};
use tokio::sync::Mutex;
use tracing::instrument;

use crate::{log, Context, Result};

/// Joins the author's voice channel based on the given context.
#[instrument(skip(ctx), fields(author=%ctx.author(), guild=?ctx.guild_id()))]
pub async fn join_author(ctx: &Context<'_>) -> Result<Arc<Mutex<Call>>> {
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

    let empty_leaver = EmptyChannelLeaver {
        call: call.clone(),
        channel_id,
        ctx: ctx.discord().to_owned(),
    };

    let dc_stop = StopOnDisconnect { call: call.clone() };
    let mut call_lock = call.lock().await;
    call_lock.add_global_event(
        Event::Periodic(std::time::Duration::from_secs(5 * 60), None),
        empty_leaver,
    );
    call_lock.add_global_event(Event::Core(songbird::CoreEvent::DriverConnect), dc_stop);

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

struct EmptyChannelLeaver {
    ctx: serenity::Context,
    channel_id: serenity::ChannelId,
    call: Arc<Mutex<Call>>,
}

#[async_trait]
impl EventHandler for EmptyChannelLeaver {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        let channel = unwrap_result_cancel!(
            self.channel_id.to_channel(&self.ctx).await,
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

struct StopOnDisconnect {
    call: Arc<Mutex<Call>>,
}

#[async_trait]
impl EventHandler for StopOnDisconnect {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        let call_lock = self.call.lock().await;
        call_lock.queue().stop();
        Some(Event::Cancel)
    }
}
