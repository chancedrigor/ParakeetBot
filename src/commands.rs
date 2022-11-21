mod play;
mod skip;
mod stop;
use std::vec;

use crate::{Data, Error};

pub type Command = poise::Command<Data, Error>;

pub fn list() -> Vec<Command> {
    vec![play::play(), skip::skip(), stop::stop()]
}
