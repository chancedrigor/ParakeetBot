//! Misc

pub mod call;
pub mod events;
pub mod youtube;

use std::time::Duration;

/// Helper function to format a duration.
pub fn format_duration(dur: &Duration) -> String {
    let total_secs = dur.as_secs();
    let total_mins = total_secs / 60;

    let hours = total_mins / 60;
    let mins = total_mins % 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("[{hours:02}h:{mins:02}m:{secs:02}s]")
    } else {
        format!("[{mins:02}m:{secs:02}s]")
    }
}
