//! Configuration for running this bot.

use std::collections::HashSet;

use poise::Framework;
use serde::Deserialize;
use serde::Serialize;
use serenity::GuildId;
use serenity::UserId;

use crate::error::ConfigError;
use crate::serenity;

/// The path to the config file
const CONFIG_PATH: &str = "config.toml";

/// Settings read from [CONFIG_PATH] that modify bot behavior.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Token needed to use a bot account.
    discord_token: String,

    /// See [LoggingConfig]
    logging: LoggingConfig,

    /// Useful developer specific configs.
    dev_utils: DevConfig,
}

impl Config {
    /// Tries to read [CONFIG_PATH] to extract a [Config].
    /// If a file doesn't exists, create the default config file and returns error.
    /// If a file exists but is empty, re-write the default values and return error.
    /// If a file exists but is incomplete, show error and don't change files.
    /// If a file exists and is complete, read file to create a config.
    /// If file existance is indeterminent (e.g. missing permissions), return error.
    pub fn read() -> Result<Config, ConfigError> {
        let file = std::fs::read_to_string(CONFIG_PATH);

        match file {
            // Config file found
            Ok(content) => {
                // Write default values to file if it's empty.
                if content.trim().is_empty() {
                    write_file(Config::default())?;
                    Err(ConfigError::InvalidConfig {
                        reason: format!("Empty config file! Rewriting {CONFIG_PATH} ..."),
                    })
                } else {
                    // If deserialization fails, return error describing the mistake.
                    let to_toml = toml::Deserializer::new(&content);
                    let result: Result<Config, _> = serde_path_to_error::deserialize(to_toml);

                    result.map_err(|error| ConfigError::InvalidConfig {
                        reason: error.to_string(),
                    })
                }
            }
            // File not found or other filesystem error
            Err(file_error) => {
                match file_error.kind() {
                    // If file doesn't exist, create default config file.
                    std::io::ErrorKind::NotFound => {
                        let action = format!("Creating {CONFIG_PATH}...");
                        write_file(Config::default())?;
                        Err(ConfigError::MissingConfig { action_msg: action })
                    }
                    // If we can't determine that config file exist: log error and use default settings (no file writes)
                    _ => Err(ConfigError::IoError(file_error)),
                }
            }
        }
    }

    /// Basic sanity check for if a token was given.
    pub fn token(&self) -> Result<&String, ConfigError> {
        let default_token = Config::default().discord_token;
        let given_token = &self.discord_token;

        let is_empty = given_token.is_empty();
        let contains_default = given_token.contains(&default_token);

        let sanity_check: bool = !is_empty && !contains_default;

        if sanity_check {
            Ok(&self.discord_token)
        } else {
            Err(ConfigError::InvalidConfig {
                reason: "Missing discord token".to_string(),
            })
        }
    }

    /// Construct a bug notification notify list based on the config.
    /// Wrapper for [NotifyConfig::notify_list]
    pub fn notify_list<U, E>(&self, fw: &Framework<U, E>) -> HashSet<UserId> {
        self.dev_utils.notifications.notify_list(fw)
    }

    /// Getter for log_dir.
    /// TODO: Path validation?
    pub fn log_dir(&self) -> &str {
        &self.logging.log_dir
    }

    /// Is debug mode enabled for console logs
    pub fn console_debug(&self) -> bool {
        self.logging.console_debug
    }

    /// Is file logging enabled.
    pub fn logs_enabled(&self) -> bool {
        self.logging.logs_enabled
    }

    pub fn dev_guild(&self) -> Option<GuildId> {
        self.dev_utils.dev_guild
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discord_token: "put_token_here".to_string(),

            logging: LoggingConfig {
                console_debug: false,
                logs_enabled: true,
                log_dir: "logs".to_string(),
            },

            dev_utils: DevConfig {
                dev_guild: None,
                notifications: NotifyConfig {
                    enabled: false,
                    add_owners: true,
                    userids: vec![],
                },
            },
        }
    }
}

// /// Represents possible log levels to filter messages shown.
// #[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
// #[serde(rename_all = "lowercase")]
// #[allow(clippy::missing_docs_in_private_items)]
// enum LogLevel {
//     #[serde(alias = "false", alias = "none", alias = "no")]
//     Off,
//     Error,
//     Warn,
//     #[default]
//     Info,
//     Debug,
//     Trace,
// }

// impl From<LogLevel> for TracingLevel {
//     fn from(val: LogLevel) -> Self {
//         match val {
//             LogLevel::Off => TracingLevel::OFF,
//             LogLevel::Error => TracingLevel::ERROR,
//             LogLevel::Warn => TracingLevel::WARN,
//             LogLevel::Info => TracingLevel::INFO,
//             LogLevel::Debug => TracingLevel::DEBUG,
//             LogLevel::Trace => TracingLevel::TRACE,
//         }
//     }
// }

/// Configs for
#[derive(Debug, Serialize, Deserialize)]
struct LoggingConfig {
    /// Print debug traces to console?
    console_debug: bool,
    /// Enable writing to log file?
    logs_enabled: bool,
    /// Directory to store log files
    log_dir: String,
}

/// Optional configs to enable developer-specific behavior.
#[derive(Debug, Serialize, Deserialize)]
struct DevConfig {
    /// Optional guild to automatically update commands quickly.
    #[serde(serialize_with = "serialize_opt", deserialize_with = "deserialize_opt")]
    dev_guild: Option<GuildId>,
    /// See [NotifyConfig]
    notifications: NotifyConfig,
}

/// Configs for notification behavior when encountering unexpected errors.
#[derive(Debug, Serialize, Deserialize)]
struct NotifyConfig {
    /// Enable this behavior or not. (bot sends a private message)
    enabled: bool,
    /// Whether to automatically add owners to the notify list.
    add_owners: bool,
    /// Additional users to add to the notify list.
    userids: Vec<UserId>,
}

impl NotifyConfig {
    /// Construct a bug notification notify list based on the config.
    fn notify_list<U, E>(&self, fw: &Framework<U, E>) -> HashSet<UserId> {
        let mut notify_list = HashSet::new();

        // If disabled, don't add anyone to the list.
        if !self.enabled {
            return notify_list;
        }

        // Add bot owners if enabled
        if self.add_owners {
            let owners = &fw.options().owners;
            for userid in owners {
                notify_list.insert(*userid);
            }
        }

        // Add users in config
        for userid in &self.userids {
            notify_list.insert(*userid);
        }

        notify_list
    }
}

/// Write the given config to [CONFIG_PATH].
/// If an error occurs, it is logged and nothing happens.
fn write_file(config: Config) -> Result<(), ConfigError> {
    use std::fs::write;

    let content = toml::to_string_pretty(&config).expect("config serialization can't fail");
    write(CONFIG_PATH, content).map_err(ConfigError::IoError)
}

fn deserialize_opt<'de, D>(deserializer: D) -> Result<Option<GuildId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_str(OptVisitor)
}

fn serialize_opt<T, S>(val: &Option<T>, ser: S) -> Result<S::Ok, S::Error>
where
    T: serde::Serialize,
    S: serde::Serializer,
{
    match val {
        Some(v) => v.serialize(ser),
        None => ser.serialize_str(""),
    }
}

struct OptVisitor;

impl<'de> serde::de::Visitor<'de> for OptVisitor {
    type Value = Option<GuildId>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid guild id")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v {
            "" => Ok(None),
            _ => {
                let num: u64 = v.parse().map_err(|_| E::custom("not u64"))?;
                Ok(Some(GuildId::new(num)))
            }
        }
    }
}
