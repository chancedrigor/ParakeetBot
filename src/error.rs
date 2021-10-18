use serenity::model::id::UserId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("<@{0}> not in voice channel.")]
    NotInVoice(UserId),
    #[error("Missing '{0}' argument.")]
    MissingArg(String),
    #[error("Malformed '{0} argument.")]
    MalformedArg(String),
    #[error("Missing enviromental value: '{0}'.")]
    MissingEnv(String),
}
