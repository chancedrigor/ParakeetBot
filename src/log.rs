/*! Bot logging (mostly re-exports and utility functions).*/

pub use color_eyre::eyre::{eyre, WrapErr};
use poise::{BoxFuture, FrameworkError};
pub use tracing::*;

use crate::{Data, Error, Result};

/// Setup format layers, tracing subscribers, and installs tracing.
pub(super) fn install_tracing() -> Result<()> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let fmt_layer = fmt::layer().without_time().compact()
    // .with_target(false)
    ;
    let filter_layer = EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    Ok(())
}

/// Unwraps a result; logs an error to console instead of panicking on error.
fn ok_or_console<T, E>(res: std::result::Result<T, E>)
where
    E: std::error::Error,
{
    if let Err(e) = res {
        error!("{e:#}")
    }
}

/// Logs user errors back to users as an ephemeral message.
#[instrument]
pub fn log_to_user(err: FrameworkError<Data, Error>) -> BoxFuture<()> {
    Box::pin(async move {
        match err {
            FrameworkError::Command { error, ctx } => {
                // These should only log user errors
                let res = ctx
                    .send(|b| {
                        b.content(format!("{error}"))
                            .allowed_mentions(|f| f.empty_users())
                            .ephemeral(true)
                    })
                    .await;
                ok_or_console(res)
            }
            FrameworkError::CooldownHit {
                remaining_cooldown,
                ctx,
            } => {
                let cmd_name = &ctx.command().name;
                let res = ctx
                    .send(|b| {
                        b.ephemeral(true).content(format!(
                            "{cmd_name} is on cooldown for {:.2} s.",
                            remaining_cooldown.as_secs_f32()
                        ))
                    })
                    .await;
                ok_or_console(res)
            }
            _ => error!("{err:#}"),
        }
    })
}
