//! Implements the `/play` command.
//!
//! This command takes one argument `query` which can be a search query or an url.
//! In either case, the bot will try to autocomplete the search.
//!

use std::str::FromStr;
use std::time::Duration;

use poise::CreateReply;
use serenity::AutocompleteChoice;
use serenity::CreateEmbed;
use songbird::input::AuxMetadata;
use songbird::input::Input;
use songbird::input::YoutubeDl;
use tokio::time::sleep;
use tracing::instrument;

use crate::data::GetData;
use crate::error::UserError;
use crate::lib;
use crate::lib::call;
use crate::lib::youtube;
use crate::serenity;
use crate::Context;
use crate::ParakeetError;
use youtube::SearchResult;

/// Types of queries that are derived from user
#[derive(Clone, Debug)]
enum Query {
    /// A fully qualified url to a youtube video
    YoutubeURL(String),
    /// A string query for a youtube search
    YoutubeSearch(String),
    /// A fully qualified url to something other than youtube, might not work
    Other(String),
    /// Explicitly marked as not supported
    Unsupported,
}

impl FromStr for Query {
    type Err = ParakeetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check if input is an url
        if let Ok(url) = s.parse::<url::Url>() {
            // Check the domain
            match url.domain() {
                Some("www.youtube.com" | "www.youtu.be") => Ok(Query::YoutubeURL(s.to_string())),
                Some("open.spotify.com") | Some("spotify.com") => Ok(Query::Unsupported),
                Some(_) | None => Ok(Query::Other(s.to_string())),
            }
        } else {
            // If not url, input is a youtube query
            Ok(Query::YoutubeSearch(s.to_string()))
        }
    }
}

/// Autocompletes 'partial' arguments in a play command.
/// If `input` is a valid url, this will autocomplete into one choice that links to that url
/// If `input` is a string query, this will autocomplete into multiple choices, each corresponding
/// to unique youtube search options.
#[instrument(skip(_ctx))]
async fn autocomplete_query(_ctx: Context<'_>, input: &str) -> Vec<AutocompleteChoice> {
    // Don't start until input isn't empty.
    if input.is_empty() {
        return vec![];
    };

    // Small delay to prevent unnecessary autocompletions.
    sleep(Duration::from_millis(600)).await;

    tracing::debug!("Autocompleting for '{input}'");

    // If input is an url, autocomplete one choice
    if let Ok(url) = url::Url::parse(input) {
        match youtube::search_link(url).await {
            Ok(SearchResult { name, url }) => {
                return vec![AutocompleteChoice::new(name, url)];
            }
            Err(e) => {
                tracing::error!("{input} was a valid URL but encountered:\n{e}");
            }
        };
    };

    match youtube::search_query(input, 5).await {
        Ok(results) => {
            return results
                .into_iter()
                .map(|SearchResult { name, url }| AutocompleteChoice::new(name, url))
                .collect()
        }
        Err(e) => {
            tracing::error!("Tried to yt-search for '{input} but encountered:\n{e}");
        }
    };

    vec![]
}

/// Plays from the given link or does a youtube search on the query.
#[instrument(skip(ctx))]
#[poise::command(slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Youtube query or url"]
    #[autocomplete = "autocomplete_query"]
    query: Query,
) -> Result<(), ParakeetError> {
    // Make a yt-search if we don't have an url
    let input_url = match query {
        Query::YoutubeURL(url) | Query::Other(url) => url,
        Query::YoutubeSearch(q) => {
            let search_result = youtube::search_best(q).await?;
            search_result.url
        }
        Query::Unsupported => Err(UserError::UnsupportedPlatform)?,
    };

    tracing::debug!("Resolved Url: {input_url}");

    let http_client = ctx.http_client().await;

    // Join the user's call
    let call = call::join_author(&ctx).await?;

    ctx.defer().await?;

    // Get input and it's metadata.
    let mut input: Input = YoutubeDl::new(http_client, input_url.clone()).into();
    let meta = input.aux_metadata().await?;

    let _handle = call::enqueue(&ctx, &call, input).await?;

    // Build the reply and send it
    let reply = play_reply(&meta);
    ctx.send(reply).await?;

    Ok(())
}

/// Plays from the given link or does a youtube search on the query.
#[instrument(skip(ctx))]
#[poise::command(slash_command, guild_only, rename = "playfile")]
pub async fn play_file(
    ctx: Context<'_>,
    #[description = "Attachment or file."] file: serenity::Attachment,
) -> Result<(), ParakeetError> {
    let input_url = file.url;

    tracing::debug!("Url: {input_url}");

    let http_client = ctx.http_client().await;

    // Join the user's call
    let call = call::join_author(&ctx).await?;

    // Get input and it's metadata.
    let mut input: Input = YoutubeDl::new(http_client, input_url.clone()).into();
    let meta = input.aux_metadata().await?;

    let _handle = call::enqueue(&ctx, &call, input).await?;

    // Build the reply and send it
    let reply = play_reply(&meta);
    ctx.send(reply).await?;

    Ok(())
}

/// Create a reply based on the metadata of the input.
fn play_reply(meta: &AuxMetadata) -> CreateReply {
    let title = meta.title.clone().unwrap_or("<MISSING TITLE>".to_string());

    let mut embed = CreateEmbed::default().title(title);

    // Make title link to url if available.
    if let Some(url) = meta.source_url.clone() {
        embed = embed.url(url);
    }

    if let Some(thumbnail) = meta.thumbnail.clone() {
        embed = embed.thumbnail(thumbnail)
    }

    // Add various fields if they are available.
    if let Some(dur) = meta.duration {
        embed = embed.field("Duration", lib::format_duration(&dur), true);
    }
    if let Some(date) = meta.date.clone() {
        embed = embed.field("Date", date, true);
    }
    if let Some(channel) = meta.channel.clone() {
        embed = embed.field("Channel", channel, true);
    }

    CreateReply::default().embed(embed)
}
