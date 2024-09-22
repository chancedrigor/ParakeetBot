//! This module contains everything relating to [Data].

mod queue_metadata;

use std::collections::HashMap;
use std::collections::HashSet;

use std::sync::Arc;

use reqwest::Client;
use serenity::GuildId;
use serenity::UserId;
use tokio::sync::Mutex;

use crate::error::UserError;
use crate::serenity;
use crate::Context;
pub use queue_metadata::QueueMeta;
pub use queue_metadata::TrackMetadata;

/// Convenience type alias for [UserData]
type UserDataRef = Arc<Mutex<UserData>>;

/// Convenience type alias for [GuildData]
type GuildDataRef = Arc<Mutex<GuildData>>;

/// The data kept between shards
#[derive(Debug, Default)]
pub struct Data {
    /// List of users to send bug notifications
    pub notify_list: HashSet<UserId>,
    /// Per-User data
    pub user_data: Mutex<HashMap<UserId, UserDataRef>>,
    /// Per-Guild data
    pub guild_data: Mutex<HashMap<GuildId, GuildDataRef>>,
}

#[derive(Debug, Default)]
pub struct UserData {}

/// Data stored on a per=guild basis.
#[derive(Debug, Default)]
pub struct GuildData {
    /// Metadata of tracks in queue, uses an [Arc] internally
    pub queue_metadata: QueueMeta,
}

/// Key to store a [Client] in a [TypeMapKey]
pub struct HttpKey;
impl serenity::prelude::TypeMapKey for HttpKey {
    type Value = Client;
}

/// Is able to get an [UserData] and [Client].
pub trait GetData {
    #[allow(dead_code)]
    /// Returns a reference to [UserData].
    async fn user_data(&self) -> UserDataRef;
    /// Returns a [Client].
    async fn http_client(&self) -> Client;
    /// Returns a reference to [GuildData]. Errors if not in a guild.
    async fn guild_data(&self) -> Result<GuildDataRef, UserError>;
}

impl GetData for Context<'_> {
    async fn user_data(&self) -> UserDataRef {
        let user = self.author().id;
        let mut map = self.data().user_data.lock().await;

        match map.get(&user) {
            Some(user_data) => user_data.clone(),
            None => {
                let default_data: UserDataRef = Default::default();
                map.insert(user, default_data.clone());
                default_data
            }
        }
    }

    async fn http_client(&self) -> Client {
        self.serenity_context()
            .data
            .read()
            .await
            .get::<HttpKey>()
            // Client internally uses an Arc, so this is cheap to clone
            .cloned()
            .expect("Expected http client")
    }

    async fn guild_data(&self) -> Result<GuildDataRef, UserError> {
        let guild = self.guild_id().ok_or(UserError::GuildOnly)?;
        let mut map = self.data().guild_data.lock().await;

        match map.get(&guild) {
            Some(data) => Ok(data.clone()),
            None => {
                let default_data: GuildDataRef = Default::default();
                map.insert(guild, default_data.clone());
                Ok(default_data)
            }
        }
    }
}
