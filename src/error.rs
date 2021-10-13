use serenity::model::id::UserId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("<@{0}> not in voice channel.")]
    NotInVoice(UserId),
}
