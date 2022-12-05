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
    let format: &str = &[
        "%(title)",            // title
        ".50",                 // max title length
        "s",                   // convert to string
        " [",                  // aesthetics
        "%(duration_string)s", // duration, convert to string
        "] - '",               // aesthetics
        "%(channel)s ",        // uploader, convert to string
        "' ",                  // aesthetics
        "%(view_count)",       // view_count
        "D",                   // add decimal suffixes (e.g 10M, 200k, ...)
        " views",              // to be extra clear on the above
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
        .args(&ytdlp_args)
        .stdin(std::process::Stdio::null())
        .output()
        .await?;

    let out_string = String::from_utf8(ytdlp_output.stdout)?;
    let mut results = Vec::new();

    let mut iter = out_string.split_terminator('\n');
    loop {
        match (iter.next(), iter.next()) {
            (Some(k), Some(v)) => results.push((k.to_string(), v.parse()?)),
            _ => break,
        };
    }

    Ok(results)
}
