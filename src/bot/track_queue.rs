//! Implements a track queue as a wrapper around [TrackQueue] with additional functionality.
//!
//! Where possible, these functions should be used over [songbird]'s where possible.
use std::fmt::Display;

use songbird::tracks::TrackQueue;
pub use track::Track;

/// Wrapper struct for a [TrackQueue].
#[derive(Clone, Debug)]
pub struct Queue<'a>(&'a TrackQueue);

/// Meant to be displayed in discord as an embed.
impl Display for Queue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tracks = self.0.current_queue();
        for (i, track) in tracks.into_iter().map(Track::from).enumerate() {
            writeln!(f, "`{index}.` {track}", index = i + 1)?;
        }
        Ok(())
    }
}

impl<'a> From<&'a TrackQueue> for Queue<'a> {
    fn from(tq: &'a TrackQueue) -> Self {
        Queue(tq)
    }
}

impl<'a> From<&'a Queue<'a>> for &'a TrackQueue {
    fn from(q: &'a Queue) -> Self {
        q.0
    }
}

mod track {
    //! Implements display for [TrackHandle].

    use songbird::tracks::TrackHandle;

    use super::*;

    /// Wrapper struct for a TrackHandle
    #[derive(Clone, Debug)]
    pub struct Track(TrackHandle);

    impl From<TrackHandle> for Track {
        fn from(th: TrackHandle) -> Self {
            Track(th)
        }
    }

    impl From<Track> for TrackHandle {
        fn from(t: Track) -> Self {
            t.0
        }
    }

    impl Display for Track {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let meta = self.0.metadata();
            let title = meta.title.as_ref().unwrap();
            let url = meta.source_url.as_ref().unwrap();
            write!(f, "[{title}]({url}')")
        }
    }
}
