/*! Commands that the bot will register and respond to. */

mod play;
mod skip;
mod stop;
use std::vec;

use crate::{Data, Error};

/// Convenient type alias for [poise::Command].
pub type Command = poise::Command<Data, Error>;

/// Lists all the implemented commands
pub fn list() -> Vec<Command> {
    vec![play::play(), skip::skip(), stop::stop()]
}
