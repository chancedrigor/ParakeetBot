//! * Functionality for interfacing with youtube (e.g. searches).

use tracing::instrument;

use crate::{error::UserError, ParakeetError};

/// A youtube video with formatted metadata and its url.
pub struct SearchResult {
    /// Display name
    pub name: String,
    /// The url of source
    pub url: String,
}

/// Searches youtube for the given query.
///
/// `limit` is the max amount of results to get.
#[instrument(fields(query=query.as_ref()))]
pub async fn search_query(
    query: impl AsRef<str>,
    limit: u8,
) -> Result<Vec<SearchResult>, ParakeetError> {
    let uri = &format!("ytsearch{limit}:{}", query.as_ref());
    search(uri).await
}

/// Searches youtube for the given query.
/// Returns the first result.
/// `limit` is the max amount of results to get.
#[instrument(err, fields(query=query.as_ref()))]
pub async fn search_best(query: impl AsRef<str>) -> Result<SearchResult, ParakeetError> {
    let uri = &format!("ytsearch1:{}", query.as_ref());
    let results = search(uri).await?;
    match results.into_iter().next() {
        Some(search_result) => Ok(search_result),
        None => Err(UserError::SearchFailed {
            reason: "No results found.".to_string(),
        })?,
    }
}

/// Searches youtube for the given link.
#[instrument(err)]
pub async fn search_link(url: url::Url) -> Result<SearchResult, ParakeetError> {
    let results = search(url).await?;
    match results.into_iter().next() {
        None => Err(UserError::SearchFailed {
            reason: "No results found".to_string(),
        })?,
        Some(search_res) => Ok(search_res),
    }
}

/// Helper function that actually calls yt-dlp.
async fn search(uri: impl AsRef<str>) -> Result<Vec<SearchResult>, ParakeetError> {
    // Discord enforces a 100 char limit so we budget
    // Format is title[duration](views)-channel
    let format: &str = &[
        "%(title).60s ",          // Title, at most 60 chars
        "[%(duration_string)s] ", // Duration in '[HH:MM:SS]' format, at most 10 chars
        // View count in '(dddc views)' format, at most 12 chars
        "(%(view_count)D ", // add decimal suffixes (e.g 10M, 200k, ...)
        " views)",          // add ' views' as suffix
        "- ",
        "%(channel).14s", // Channel name in '-name' format, max 15 chars
    ]
    .concat();

    let ytdlp_args = [
        "--no-warnings",
        "--ignore-config",
        "--flat-playlist",
        "--print",
        format,
        "--print",
        "webpage_url",
        uri.as_ref(),
    ];

    let ytdlp_output = tokio::process::Command::new("yt-dlp")
        .args(ytdlp_args)
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .map_err(ParakeetError::IoError)?;

    // Convert `Output` into a string, this should never fail
    let out_string = String::from_utf8(ytdlp_output.stdout).map_err(ParakeetError::Utf8Error)?;

    let mut iter = out_string.split('\n');
    let mut results = Vec::new();

    while let (Some(name), Some(url)) = (iter.next(), iter.next()) {
        results.push(SearchResult {
            name: name.to_string(),
            url: url.to_string(),
        });
    }

    Ok(results)
}
