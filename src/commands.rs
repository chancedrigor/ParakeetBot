mod play;
mod skip;
use std::vec;

use crate::{Data, Error};

pub type Command = poise::Command<Data, Error>;

pub fn list() -> Vec<Command> {
    vec![play::play(), skip::skip()]
}
