//! Bot commands.

mod play;
mod queue;
mod skip;
mod stop;

use crate::{Data, ParakeetError};

/// Convenient type alias for [poise::Command].
pub type Command = poise::Command<Data, ParakeetError>;

/// Lists all the implemented commands
pub fn list() -> Vec<Command> {
    vec![
        play::play(),
        play::play_file(),
        skip::skip(),
        stop::stop(),
        queue::queue(),
    ]
}
