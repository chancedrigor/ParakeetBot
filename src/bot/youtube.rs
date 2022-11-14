use log::instrument;
use url::Url;

use crate::{log, Result};

type SearchResults = Vec<(String, Url)>;

/// Gets the metadata of all the videos in a youtube search.
/// `limit` is the max amount of results to get.
#[instrument(fields(query=query.as_ref()))]
pub async fn search(query: impl AsRef<str>, limit: u8) -> Result<SearchResults> {
    let query = query.as_ref();
    let uri = &format!("ytsearch{limit}:{query}");

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
        uri,
    ];

    let ytdlp_output = tokio::process::Command::new("yt-dlp")
        .args(&ytdlp_args)
        .stdin(std::process::Stdio::null())
        .output()
        .await?;

    let out_string = String::from_utf8(ytdlp_output.stdout)?;
    let mut results = SearchResults::new();

    let mut iter = out_string.split_terminator('\n');
    loop {
        match (iter.next(), iter.next()) {
            (Some(k), Some(v)) => results.push((k.to_string(), v.parse()?)),
            _ => break,
        };
    }

    Ok(results)
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_search() -> Result<()> {
        let out = search("One last kiss", 5).await?;
        println!("{out:?}");
        Ok(())
    }
}
