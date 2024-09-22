//! Errors used in this crate.

use std::time::Duration;

use thiserror::Error;

use crate::lib::format_duration;
use crate::serenity;

/// Helper macro to format Option<String> types
macro_rules! format_opt {
    ($opt:expr, $some:expr, $none:expr) => {{
        match $opt {
            Some(s) => format!($some, s),
            None => format!($none),
        }
    }};
}

/// Contains all error types produced in this crate.
#[derive(Error)]
pub enum ParakeetError {
    /// Errors that are shown to users, see [UserError]
    #[error(transparent)]
    UserError(#[from] UserError),
    /// Errors relating to configs, see [ConfigError]
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    /// Errors relating to IO
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// Conversion to utf8 failed.
    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),
    /// There was an unexpected panic somewhere.
    /// This is a last-resort for graceful shutdown and should never be constructed in code.
    /// It only exists to translate a [poise::FrameworkError::CommandPanic].
    #[error("{}", format_opt!(payload, "Panic! Payload: {}", "Panic! Payload: None"))]
    Panic {
        /// A payload from the panic if there is one
        payload: Option<String>,
    },
    /// Unexpected mismatch of arg types, most likely from a delay when updating commands on discord.
    #[error("Command structure mismatch! {description}")]
    CommandStructureMismatch {
        /// A helpful message describing the problem
        description: String,
    },
    /// Command check failed!
    #[error("{}", format_opt!(reason, "Denied! Reason: '{}'", "Denied!"))]
    CheckFailed {
        /// Why it failed, if there is a reason.
        reason: Option<String>,
    },
    /// Errors from [serenity]
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    /// Errors from [songbird]
    #[error(transparent)]
    Songbird(#[from] songbird::error::JoinError),
    /// Something wasn't initialized correctly.
    #[error("Setup was incorrect. Reason: {reason}")]
    MissingFromSetup { reason: String },
    /// Failed to get metadata for a track.
    #[error(transparent)]
    MetadataError(#[from] songbird::input::AuxMetadataError),
    /// Track manipulation error
    #[error(transparent)]
    ControlError(#[from] songbird::tracks::ControlError),
}

/// Make debug implementation return the [std::fmt::Display] implementation to
/// show nice errors when returning from main.
impl std::fmt::Debug for ParakeetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

/// These are "user-facing" errors that are shown back to them.
/// These do not indicate unexpected behavior and should never cause a panic.
#[derive(Error, Debug)]
pub enum UserError {
    /// Could not find the user's voice channel.
    #[error("Couldn't find you in a voice channel!")]
    NotInVoice,
    /// Could not find the user's guild.
    #[error("Couldn't find your server!")]
    NotInGuild,
    /// There was no active call in this guild.
    #[error("No active call.")]
    NoActiveCall,
    /// YT-search failed for some reason.
    #[error("Search failed: {reason}")]
    SearchFailed {
        /// Why the search failed
        reason: String,
    },
    /// User tried to use an unsupported platform.
    #[error("Unsupported platform, sorry! :(")]
    UnsupportedPlatform,
    /// User left out a required subcommand
    #[error("Missing a subcommand: {subcmds}")]
    MissingSubcommand {
        /// A list of subcommands
        subcmds: String,
    },
    /// User gave input that failed to parse.
    #[error("{}", format_opt!(input, "Failed to parse '{}'", "Failed to parse input"))]
    BadArgs {
        /// The specific part that was problematic
        input: Option<String>,
    },
    /// User tried to invoke command while it was still on cooldown.
    #[error("Cooldown: {}", format_duration(remaining_cooldown))]
    OnCooldown {
        /// Time remaining until cooldown is over
        remaining_cooldown: Duration,
    },
    /// Bot needs permissions that it doesn't have.
    #[error("Missing permissions: {missing_permissions}. Contact server admin to fix.")]
    MissingBotPermissions {
        /// List of missing permissions
        missing_permissions: serenity::Permissions,
    },
    /// User needs permissions that it doesn't have.
    #[error("{}", format_opt!(missing_permissions, "You need the permissions: '{}'", "You need permissions."))]
    MissingUserPermissions {
        /// List of missing permissions, can be `None` if fetching permissions failed.
        missing_permissions: Option<serenity::Permissions>,
    },
    /// A user, that's not the owner, tried to use an owner-only command.
    #[error("Only the owner can use that command!")]
    NotOwner,
    /// A user tried to use a guild-only command in a dm channel.
    #[error("Only usable in a server.")]
    GuildOnly,
    /// A user tried to use a pm-only command in a guild channel.
    #[error("Only usable in private messages.")]
    DmOnly,
    /// A user tried to use a nsfw-only command in a non-nsfw channel.
    #[error("Only usable in a NSFW channel. ( ͡° ͜ʖ ͡°)")]
    NsfwOnly,
    /// Queue already empty.
    #[error("Nothing in the queue!")]
    EmptyQueue,
}

/// Errors that can occur when reading/writing/parsing a config file.
/// See [crate::error].
#[derive(Error, Debug)]
pub enum ConfigError {
    /// The config file is missing and/or has invalid options/values
    #[error("Invalid configuration: {reason}")]
    InvalidConfig {
        /// The reason for why the config is invalid
        reason: String,
    },
    /// The config file doesn't exist
    #[error("Missing config file! {action_msg}")]
    MissingConfig {
        /// What was done as a result of this error, usually writing the default config file.
        action_msg: String,
    },
    /// Unable to determine if config exist, can't read, can't write, etc...
    #[error("IO error: {0}")]
    IoError(std::io::Error),
}

#[cfg(test)]
mod tests {
    #[allow(unused)]
    use super::*;

    #[test]
    #[allow(clippy::useless_format)]
    fn test_format_opt() {
        // Some case with &str
        let val = format_opt!(Some("something"), "This has {}", "This has nothing");
        let expected = format!("This has something");
        assert_eq!(val, expected);

        // None case with &str
        let val = format_opt!(None::<&str>, "This has {}", "This has nothing");
        let expected = format!("This has nothing");
        assert_eq!(val, expected);

        // Some case with String
        let val = format_opt!(
            Some("something".to_string()),
            "This has {}",
            "This has nothing"
        );
        let expected = format!("This has something");
        assert_eq!(val, expected);

        // None case with String
        let val = format_opt!(None::<String>, "This has {}", "This has nothing");
        let expected = format!("This has nothing");
        assert_eq!(val, expected);

        // Some case with String vars
        let var = Some("something".to_string());
        let val = format_opt!(&var, "This has {}", "This has nothing");
        let expected = format!("This has something");
        assert_eq!(val, expected);

        // None case with String vars
        let var: Option<String> = None;
        let val = format_opt!(&var, "This has {}", "This has nothing");
        let expected = format!("This has nothing");
        assert_eq!(val, expected);
    }
}
