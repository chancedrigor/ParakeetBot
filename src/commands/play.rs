/*!
 * Implements the `/play` command.
 *
 * This command takes one argument `query` which can be a search query or an url.
 * In either case, the bot will try to autocomplete the search.
 */

use log::instrument;
use poise::{futures_util::TryFutureExt, AutocompleteChoice};
use url::Url;

use crate::{bot, log, Context, Error, Result};

/// The input argument to a `/play` command.

#[instrument(skip(_ctx), level = "debug")]
// async fn autocomplete_query(_ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice<String>> {
async fn autocomplete_query(_ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice<Query>> {
    let min_partial_len = 2; // Min length before doing actual searches

    if partial.len() <= min_partial_len {
        log::trace!(
            "Skipping search, query length ({}) less than min ({min_partial_len}).",
            partial.len()
        );
        return Vec::new();
    };

    // If partial is an url
    if let Ok(url) = url::Url::parse(partial) {
        match bot::search_link(url).await {
            Ok((name, u)) => {
                return vec![AutocompleteChoice {
                    name,
                    value: Query::Full(u),
                }]
            }
            Err(e) => {
                log::trace!("{e}");
                return Vec::new();
            }
        };
    };

    log::trace!("Searching for '{partial}'.");
    let res = bot::search(partial, 8).await;

    match res {
        Ok(search_res) => search_res
            .into_iter()
            .map(|(name, url)| AutocompleteChoice {
                name,
                value: Query::Full(url),
            })
            .collect(),
        Err(e) => {
            log::error!("{e}");
            Vec::new()
        }
    }
}

/// Plays from the given link or does a youtube search on the query.
///
/// This won't do a search until the query is at least 2 characters long.
#[instrument(err, skip(ctx))]
#[poise::command(slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Youtube query or url"]
    #[autocomplete = "autocomplete_query"]
    query: Query,
) -> Result<()> {
    let source_url = query.try_into_url().await?;

    let reply = ctx
        .say(format!("Playing: {source_url}"))
        .map_err(|e| e.into());

    let queque_audio = async {
        let call = bot::join_author(&ctx).await?; // Join the call if not in there
        let audio_source = songbird::ytdl(&source_url).await?;

        call.lock().await.enqueue_source(audio_source);
        Ok::<(), Error>(())
    };

    // Concurrently send reply & queue song.
    let (_reply_handle, _) = tokio::try_join!(reply, queque_audio)?;
    Ok(())
}

use query::Query;

/// Implements a partial or fully resolved query
mod query {
    use poise::{async_trait, serenity_prelude as serenity, SlashArgError};
    use serde::Serialize;

    use super::*;

    /// A query, this type is returned by invokations of the `/play` command.
    #[derive(Debug, Clone, Serialize)]
    #[serde(untagged)]
    pub enum Query {
        /// A fully resolved query.
        Full(Url),
        /// An unresolved search query.
        ///
        /// This occurs when autocomplete is not able to resolve the query in time.
        /// Usually because the user entered the command quickly after typing it.
        ///
        /// The query must be resolved before playing anything.
        Partial(String),
    }

    #[async_trait]
    impl poise::SlashArgument for Query {
        fn choices() -> Vec<poise::CommandParameterChoice> {
            Vec::new()
        }

        fn create(builder: &mut serenity::CreateApplicationCommandOption) {
            builder.kind(serenity::CommandOptionType::String);
        }

        async fn extract(
            _ctx: &serenity::Context,
            _interaction: poise::ApplicationCommandOrAutocompleteInteraction<'_>,
            value: &serenity::json::Value,
        ) -> Result<Self, SlashArgError> {
            let string = value
                .as_str()
                .ok_or(SlashArgError::CommandStructureMismatch("expected string"))?;
            match Url::parse(string) {
                Ok(u) => Ok(Query::Full(u)),
                Err(_) => Ok(Query::Partial(string.to_string())),
            }
        }
    }

    impl Query {
        /// Try to convert to [url::Url].
        ///
        /// Does a search on a partial query, returning the first result.
        pub async fn try_into_url(self) -> Result<Url> {
            match self {
                Query::Full(u) => Ok(u),
                Query::Partial(query) => {
                    let search = bot::search(&query, 1).await?;
                    match search.first() {
                        None => Err(log::eyre!("No search results found for : '{query}'")),
                        Some((_, u)) => Ok(u.to_owned()),
                    }
                }
            }
        }
    }
}
