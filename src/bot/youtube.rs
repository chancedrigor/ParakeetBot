//! * Functionality for interfacing with youtube (e.g. searches).

use log::instrument;
use url::Url;

use crate::{log, Result};

/// A youtube video with formatted metadata and its url.
type SearchResult = (String, Url);

/// Searches youtube for the given query.
///
/// `limit` is the max amount of results to get.
#[instrument(fields(query=query.as_ref()))]
pub async fn search(query: impl AsRef<str>, limit: u8) -> Result<Vec<SearchResult>> {
    let uri = &format!("ytsearch{limit}:{}", query.as_ref());
    _search(uri).await
}

/// Searches youtube for the given link.
#[instrument]
pub async fn search_link(url: Url) -> Result<SearchResult> {
    _search(url)
        .await?
        .first()
        .ok_or_else(|| log::eyre!("No results found."))
        .cloned()
}

/// Helper function that actually calls yt-dlp.
async fn _search(s: impl AsRef<str>) -> Result<Vec<SearchResult>> {
    // Discord enforces a 100 char limit so we budget
    // Format is title[duration](views)-channel
    let format: &str = &[
        // Title, at most 60 chars
        "%(title)",
        ".60",
        "s",
        // Duration in '[HH:MM:SS]' format, at most 10 chars
        "[",
        "%(duration_string)s",
        "]",
        // View count in '(dddc views)' format, at most 12 chars
        "(",
        "%(view_count)",
        "D", // add decimal suffixes (e.g 10M, 200k, ...)
        " views",
        ")",
        // Channel name in '-name' format, max 15 chars
        "-",
        "%(channel).14s",
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
        s.as_ref(),
    ];

    let ytdlp_output = tokio::process::Command::new("yt-dlp")
        .args(ytdlp_args)
        .stdin(std::process::Stdio::null())
        .output()
        .await?;

    // Convert `Output` into a string
    let out_string = String::from_utf8(ytdlp_output.stdout)?;
    // Initialize accumulator for search results
    let mut results = Vec::new();

    let mut iter = out_string.split_terminator('\n');

    // Iterate until there are no more matched pairs of (metadata, url).
    while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
        results.push((k.to_string(), v.parse()?));
    }

    Ok(results)
}
