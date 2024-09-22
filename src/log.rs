//! Logging functionality and error reporting.
//! The logging library of choice is [tracing].

use bon::builder;
use itertools::Itertools;
use poise::BoxFuture;
use poise::CreateReply;
use poise::FrameworkError;
use serenity::CreateMessage;
use tracing::debug;
use tracing::error;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::error::UserError;
use crate::serenity;
use crate::Config;
use crate::Context;
use crate::Data;
use crate::ParakeetError;

/// The name of this crate, used to set filter target.
const THIS_CRATE: &str = env!("CARGO_CRATE_NAME");

/// Setup format layers, tracing subscribers, and installs tracing.
pub(super) fn install_tracing(config: &Config) -> Option<WorkerGuard> {
    // Uses local time.
    let timer = fmt::time::ChronoLocal::rfc_3339();

    // Set which traces are tracked.
    // By default, all INFO traces and above are shown.
    let target = if config.console_debug() {
        Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(THIS_CRATE, LevelFilter::DEBUG)
    } else {
        Targets::new().with_default(LevelFilter::INFO)
    };

    // Compose the layer that prints traces to stdout
    let console_layer = if config.console_debug() {
        // Debug layer
        fmt::layer()
            .with_ansi(true)
            .with_file(true)
            .with_level(true)
            .with_line_number(true)
            .with_target(true)
            .with_timer(timer.clone())
            .pretty()
            .with_filter(target.clone())
    } else {
        // Default layer
        fmt::layer()
            .with_ansi(true)
            .with_file(false)
            .with_level(true)
            .with_line_number(false)
            .with_target(true)
            .with_timer(timer.clone())
            .pretty()
            .with_filter(target.clone())
    };

    // Compose the layer that writes logs and get a guard for the writer.
    // Output is similar to console logs with a few changes (see below).
    let (log_layer, guard) = if config.logs_enabled() {
        // Get the directory to store logs.
        let dir = config.log_dir();

        // Put file logs in `log_dir` directory as "{THIS_CRATE}.log.{TIMESTAMP}" on an hourly basis.
        let prefix_format = format!("{THIS_CRATE}.log");
        let appender = tracing_appender::rolling::hourly(dir, prefix_format);

        // Create the writer and writer guard.
        let (writer, guard) = tracing_appender::non_blocking(appender);

        // Construct the layer.
        let layer = if config.console_debug() {
            // Debug layer
            fmt::layer()
                .with_ansi(false)
                .with_file(true)
                .with_level(true)
                .with_line_number(true)
                .with_target(true)
                .with_timer(timer)
                .with_writer(writer)
                .compact()
                .with_filter(target)
        } else {
            // Default layer
            fmt::layer()
                .with_ansi(false)
                .with_file(false)
                .with_level(true)
                .with_line_number(false)
                .with_target(true)
                .with_timer(timer)
                .with_writer(writer)
                .compact()
                .with_filter(target)
        };

        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    // Add all the layers and initialize them.
    tracing_subscriber::registry()
        .with(console_layer)
        .with(log_layer)
        .init();

    guard
}

/// Defines various behaviors for how to handle errors.
/// Triggers an [ephemeral_reply] on [ParakeetError::UserError].
/// Triggers a [notify_bug] on unexpected errors.
pub fn handle_framework_error(err: FrameworkError<Data, ParakeetError>) -> BoxFuture<()> {
    let handler = async move {
        match err {
            // ---
            // This section includes all errors that should be invisible to users.
            // ---
            FrameworkError::Setup {
                error,
                // framework,
                // data_about_bot,
                ..
            } => error!("Error during startup: {error}"),
            FrameworkError::EventHandler {
                error,
                event,
                // framework,
                ..
            } => error!("Error while handling event. Event: {event:#?} Error:{error}"),

            // ---
            // This section includes errors that users see but are not logged as error!
            // For example, a user that uses a command while still on cooldown is shown an 'error', but
            // no unexpected behavior occured.
            // ---
            FrameworkError::SubcommandRequired { ctx } => {
                let subcmds = ctx
                    .command()
                    .subcommands
                    .iter()
                    .map(|s| s.name.as_str())
                    .join(", ");
                let user_error = UserError::MissingSubcommand { subcmds };

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            // This branch specifically handles only errors that are UserError. Other types are handled in the
            // next section bellow.
            FrameworkError::Command {
                error: ParakeetError::UserError(user_error),
                ctx,
                ..
            } => {
                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::ArgumentParse {
                error, input, ctx, ..
            } => {
                let user_error = UserError::BadArgs { input };

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .add_info(error.to_string())
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::CooldownHit {
                remaining_cooldown,
                ctx,
                ..
            } => {
                let user_error = UserError::OnCooldown { remaining_cooldown };

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::MissingBotPermissions {
                missing_permissions,
                ctx,
                ..
            } => {
                let user_error = UserError::MissingBotPermissions {
                    missing_permissions,
                };

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::MissingUserPermissions {
                missing_permissions,
                ctx,
                ..
            } => {
                let user_error = UserError::MissingUserPermissions {
                    missing_permissions,
                };

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::NotAnOwner { ctx, .. } => {
                let user_error = UserError::NotOwner;

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::GuildOnly { ctx, .. } => {
                let user_error = UserError::GuildOnly;

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::DmOnly { ctx, .. } => {
                let user_error = UserError::DmOnly;

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::NsfwOnly { ctx, .. } => {
                let user_error = UserError::NsfwOnly;

                Response::builder()
                    .ctx(&ctx)
                    .reply(user_error.to_string())
                    .source(user_error)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                let error = ParakeetError::CheckFailed {
                    reason: error.map(|e| e.to_string()),
                };

                Response::builder()
                    .ctx(&ctx)
                    .reply(error.to_string())
                    .source(error)
                    .build()
                    .send()
                    .await;
            }

            // ---
            // This section includes errors that users see and are logged as error!
            // For example, a user uses a command and there was an unexpected panic during command execution.
            // The user is told that something wrong has happened. These are unexpected errors and should be fixed.
            // Additionally, all of these should cause a bug notification.
            // ---
            FrameworkError::Command { error, ctx, .. } => {
                Response::builder()
                    .ctx(&ctx)
                    .reply("Something went wrong... A bug report has been sent.")
                    .source(error)
                    .notify(true)
                    .is_error(true)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::CommandPanic { payload, ctx, .. } => {
                let error = ParakeetError::Panic { payload };

                Response::builder()
                    .ctx(&ctx)
                    .reply("Something went horribly wrong... A bug report has been sent.")
                    .source(error)
                    .notify(true)
                    .is_error(true)
                    .build()
                    .send()
                    .await;
            }
            FrameworkError::CommandStructureMismatch {
                description, ctx, ..
            } => {
                let error = ParakeetError::CommandStructureMismatch {
                    description: description.to_string(),
                };

                Response::builder()
                .ctx(&ctx.into())
                .reply("Command structure mismatch. Please wait until discord catches up to a bot update.")
                .source(error)
                .notify(true)
                .is_error(true)
                .build()
                .send()
                .await;
            }

            // ---
            // This section includes errors that should be unreachable.
            // No response is necessary but an error! log can be written.
            // ---
            FrameworkError::UnknownCommand { .. } => {
                error!("Prefix commands are not supported.")
            }
            FrameworkError::UnknownInteraction {
                // ctx,
                // framework,
                interaction,
                ..
            } => {
                let name = &interaction.data.name;
                error!("Received unknown interaction: {name}")
            }
            FrameworkError::DynamicPrefix { .. } => {
                error!("Dynamic prefixes are not supported.")
            }
            _ => error!("The dev must have forgotten something..."),
        }
    };

    Box::pin(handler)
}

/// Sends an ephemeral reply to the [Context] author.
async fn ephemeral_reply(ctx: &Context<'_>, content: impl Into<String>) {
    let reply = CreateReply::default().ephemeral(true).content(content);
    if let Err(e) = ctx.send(reply).await {
        error!("Failed to send ephemeral reply. {e}")
    };
}

/// Sends a notification (via private message) to users in [notify_bugs](crate::config::NotifyConfig).
/// If message fails, only log and don't retry.
async fn notify_bug(ctx: &Context<'_>, content: impl Into<String>) {
    let message = CreateMessage::new().content(content);

    let notify_list = &ctx.data().notify_list;
    for user in notify_list {
        if let Err(e) = user.direct_message(ctx, message.clone()).await {
            error!("Failed to send bug notification. {e}");
        }
    }
}

/// Helper function to create debug information from [Context]
fn debug_info(ctx: &Context) -> String {
    let user = &ctx.author().name;
    let cmd = &ctx.command().name;
    let user_input = ctx.invocation_string();
    format!("{user} tried to use {cmd} with {user_input}.")
}

/// Structured response to errors.
/// Always logs as at least [debug level](tracing::debug), but is upgraded to
/// [error level](tracing::error) if `error` is `Some(...)`.
/// Additionally, notify messages are accompanied by [debug info](debug_info).
#[derive(bon::Builder)]
#[builder(on(String, into))]
struct Response<'a> {
    /// The context of the response
    ctx: &'a Context<'a>,
    /// The reason for this reply, usually the error causing the response.
    #[builder(into)]
    source: ParakeetError,
    /// Optional ephemeral reply to user.
    reply: Option<String>,
    /// Additional information to log
    add_info: Option<String>,
    /// Set to `true` to log as error.
    #[builder(default = false)]
    is_error: bool,
    /// Set to `true` to send notifications of the error.
    /// Does nothing if `is_error` is false.
    #[builder(default = false)]
    notify: bool,
}

impl Response<'_> {
    /// Execute the response
    async fn send(&self) {
        let ctx = self.ctx;

        let log_message = {
            let source = &self.source;
            let add_info = self
                .add_info
                .as_ref()
                // Map `None` to "" otherwise format it to be appended to another string.
                .map_or("".to_string(), |s| format!("| {s}"));
            format!("{source} {add_info}")
        };
        if self.is_error {
            error!("{log_message}");
            if self.notify {
                // Construct and send notification message

                let dbg_info = debug_info(ctx);
                // Format of message
                let content = format!("Debug Info: {dbg_info}\n{log_message}");
                notify_bug(ctx, content).await;
            }
        } else {
            debug!("{log_message}");
        }

        // Send ephemeral reply if there is one.
        if let Some(ref reply) = self.reply {
            ephemeral_reply(ctx, reply).await;
        }
    }
}
