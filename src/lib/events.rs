//! Event handling

use std::time::Duration;

use async_trait::async_trait;
use songbird::CoreEvent;
use songbird::Event;
use songbird::EventContext;
use songbird::EventHandler;
use songbird::TrackEvent;

use super::call::get_manager;
use super::call::CallRef;
use crate::data::GetData;
use crate::data::QueueMeta;
use crate::error::UserError;
use crate::serenity;
use crate::Context;
use crate::ParakeetError;

/// Initialize global events.
/// Only initializes if a [songbird::Call] hasn't been initialized yet.
pub async fn init_global_events(ctx: &Context<'_>) -> Result<CallRef, ParakeetError> {
    let manager = get_manager(ctx).await?;
    let guild_id = ctx.guild_id().ok_or(UserError::GuildOnly)?;
    // Only init if call hasn't been initialized
    let call = {
        match manager.get(guild_id) {
            Some(call) => call,
            None => {
                let call = manager.get_or_insert(guild_id);

                tracing::info!("Initializing global events.");

                // Create the events.
                let idle_event = CheckIdle::new(&call, ctx);
                let dc_event = DisconnectStop::new(&call);
                let end_event = RemoveMeta::new(&call, ctx).await?;

                // Register them as global events.
                idle_event.register(Duration::from_secs(300)).await;
                dc_event.register().await;
                end_event.register().await;
                call
            }
        }
    };
    Ok(call)
}

/// Check if there are non-bot users in the call, if not then disconnect.
struct CheckIdle {
    /// The call to check.
    call: CallRef,
    /// Needed to find channels and guilds.
    ctx: serenity::Context,
}

impl CheckIdle {
    /// Constructor for [CheckIdle]
    fn new(call: &CallRef, ctx: &Context<'_>) -> Self {
        // Should be cheap to clone
        let ctx = ctx.serenity_context().clone();
        let call = call.clone();
        Self { call, ctx }
    }

    /// Register this as a global event
    async fn register(self, duration: Duration) {
        tracing::debug!("Registering check idle global event.");
        let call = self.call.clone();
        let mut call = call.lock().await;
        call.add_global_event(Event::Periodic(duration, None), self);
    }
}

#[async_trait]
impl EventHandler for CheckIdle {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut call = self.call.lock().await;

        if let Some(channel_id) = call
            .current_channel()
            // Convert songbird::ChannelId -> u64 -> serenity::ChannelId
            .map(|c| serenity::ChannelId::from(c.0))
        {
            // A series of conversions, each try operator (?) causes this handler
            // to retry on it's next trigger if the operator fails.
            let channel = channel_id.to_channel(&self.ctx).await.ok()?;
            let guild = channel.guild()?;
            let members = guild.members(&self.ctx).ok()?;

            // Check if there are any non-bot members.
            let has_members = members.iter().any(|m| !m.user.bot);

            if has_members {
                // With members, do nothing and retry on next trigger.
                None
            } else {
                // Otherwise, leave the call and cancel this handler.
                tracing::info!("Idle! Disconnecting from voice channel.");
                call.leave().await.ok()?;
                None
            }
        } else {
            // No channel means stop.
            call.leave().await.ok()?;
            None
        }
    }
}

/// Stop the bot when it disconnects.
/// 'Stopping' means:
/// - End anything currently playing.
/// - Reset the queue.
/// - Reset [QueueMeta]
/// - Remove other global events.
struct DisconnectStop {
    /// Reference to the call that will be dropped.
    call: CallRef,
}

impl DisconnectStop {
    /// Constructor for [DisconnectStop]
    fn new(call: &CallRef) -> Self {
        let call = call.clone();
        Self { call }
    }

    /// Register this as a global event.
    async fn register(self) {
        tracing::debug!("Registering disconnect on stop global event.");
        let call = self.call.clone();
        let mut call = call.lock().await;
        call.add_global_event(Event::Core(CoreEvent::DriverDisconnect), self);
    }
}

#[async_trait]
impl EventHandler for DisconnectStop {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        tracing::info!("Stopping on disconnect!");
        let call_lock = self.call.lock().await;
        call_lock.queue().stop();
        None
    }
}

/// Remove track metadata from queue when it's done playing.
struct RemoveMeta {
    /// Reference to call.
    call: CallRef,
    /// Reference to queue metadata.
    queue_meta: QueueMeta,
}

impl RemoveMeta {
    /// Constructor for [RemoveMeta]
    async fn new(call: &CallRef, ctx: &Context<'_>) -> Result<Self, ParakeetError> {
        let call = call.clone();
        let queue_meta = {
            let guild_data = ctx.guild_data().await?;
            let lock = guild_data.lock().await;
            lock.queue_metadata.clone()
        };
        Ok(Self { call, queue_meta })
    }

    /// Register this as a global event
    async fn register(self) {
        tracing::debug!("Registering remove metadata global event.");
        let call = self.call.clone();
        let mut call = call.lock().await;
        call.add_global_event(Event::Track(TrackEvent::End), self);
    }
}

#[async_trait]
impl EventHandler for RemoveMeta {
    async fn act(&self, _ectx: &EventContext<'_>) -> Option<Event> {
        let track = self.queue_meta.pop_front().await;
        match track {
            None => {
                tracing::error!("Tried to remove track metadata from empty queue.");
            }
            Some(meta) => {
                let title = meta.title.unwrap_or("<NO TITLE>".to_string());
                tracing::debug!("Removing metadata for {title}");
            }
        };
        None
    }
}
