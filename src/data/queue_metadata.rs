//! Stores track metadata and their display implementation.

use std::fmt::Display;
use std::{collections::VecDeque, fmt::Write};

use std::sync::Arc;
use std::time::Duration;

use delegate::delegate;
use songbird::input::Input;
use tokio::sync::Mutex;

use crate::{lib, ParakeetError};

/// Stores track metadata of the queue.
/// Internally uses an [Arc], so it's cheap to clone.
#[derive(Debug, Default, Clone)]
pub struct QueueMeta {
    #[allow(clippy::missing_docs_in_private_items)]
    inner: Arc<Mutex<VecDeque<TrackMetadata>>>,
}

impl QueueMeta {
    /// Clone the element at the front.
    pub async fn front(&self) -> Option<TrackMetadata> {
        let queue = self.inner.lock().await;
        queue.front().cloned()
    }

    delegate! {
        to self.inner.lock().await {
            /// Pop the front of the queue.
            #[await(false)]
            pub async fn pop_front(&self) -> Option<TrackMetadata>;
            /// Pop the back of the queue.
            #[await(false)]
            pub async fn pop_back(&self) -> Option<TrackMetadata>;
            /// Clear the queue.
            #[await(false)]
            pub async fn clear(&self);
            /// Add to the front of the queue.
            #[await(false)]
            pub async fn push_front(&self, meta: TrackMetadata);
            /// Add to the back of the queue.
            #[await(false)]
            pub async fn push_back(&self, meta: TrackMetadata);
        }
    }
}

impl QueueMeta {
    /// Implement "Display" on [QueueMeta]
    pub async fn display_string(&self) -> String {
        let queue = { self.inner.lock().await };

        if queue.is_empty() {
            return "Empty queue!".to_string();
        }

        let mut buffer = String::new();
        for (num, track) in queue.iter().enumerate() {
            let next_line = format!("`{num}.` {track}");

            // An embed has a limit of 4096 chars
            if buffer.len() + next_line.len() > 4096 {
                break;
            }
            writeln!(buffer, "{next_line}").expect("write to string buffer can't fail");
        }
        buffer
    }
}

/// Metadata for a track in the queue.
#[derive(Debug, Clone)]
pub struct TrackMetadata {
    /// Title of the track.
    pub title: Option<String>,
    /// Duration of the track.
    pub duration: Option<Duration>,
    /// The source's channel name.
    pub channel: Option<String>,
    /// The url to the source's thumbnail.
    pub thumbnail_url: Option<String>,
    /// Url to source
    pub url: Option<String>,
}

impl TrackMetadata {
    /// Try to get [TrackMetadata] from [Input]
    pub async fn from_input(input: &mut Input) -> Result<Self, ParakeetError> {
        let meta = input.aux_metadata().await?;
        let title = meta.title;
        let duration = meta.duration;
        let channel = meta.channel;
        let thumbnail_url = meta.thumbnail;
        let url = meta.source_url;
        Ok(TrackMetadata {
            title,
            duration,
            channel,
            thumbnail_url,
            url,
        })
    }
}

impl Display for TrackMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = self.title.clone().unwrap_or("<MISSING TITLE>".to_string());
        let channel = self.channel.clone().unwrap_or_default();
        let duration = match self.duration {
            None => String::new(),
            Some(dur) => lib::format_duration(&dur),
        };

        if let Some(source_url) = self.url.clone() {
            write!(f, "[{title} {duration} {channel}]({source_url})")
        } else {
            write!(f, "{title} {duration} {channel}")
        }
    }
}
