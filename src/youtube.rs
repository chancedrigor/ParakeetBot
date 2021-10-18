use std::process::Stdio;

use serde::Deserialize;
use serde_json::{from_str, Value};
use songbird::input::Metadata;
use tokio::process::Command as TokioCommand;
use tracing::instrument;

use crate::{error::Error, Result};

/// Gets the metadata of videos in a youtube search (`limit` is the max number of videos in the search)
///
/// This is a modified version of [`_ytdl_metadata`].
///
/// [`_ytdl_metadata`]: https://docs.rs/songbird/0.2.0/src/songbird/input/ytdl_src.rs.html#123-161
#[instrument(level = "debug", skip(query), fields(query_=query.as_ref()))]
pub async fn search(query: impl AsRef<str>, limit: u8) -> Result<Vec<Metadata>> {
    let uri = format!("ytsearch{}:{}", limit, query.as_ref());
    let ytdl_args = [
        "-J",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        uri.as_str(),
        "-o",
        "-",
    ];

    let youtube_dl_output = TokioCommand::new("youtube-dl")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await?;

    let json = std::str::from_utf8(&youtube_dl_output.stderr)?;

    // Empty json means that there were no search results
    if json.is_empty() {
        return Err(Error::NoResults(query.as_ref().to_string()).into());
    }

    let values = split_json(json)?;

    let metas: Vec<Metadata> = values
        .iter()
        .map(|s| Metadata::from_ytdl_output(s.to_owned()))
        .collect();

    // This function should never return an empty vec
    if metas.is_empty() {
        return Err(Error::NoResults(query.as_ref().to_string()).into());
    }

    Ok(metas)
}

/// Splits the json of the playlist into indivial per-video jsons.
#[instrument(level = "debug", skip_all)]
fn split_json(big_json: &str) -> Result<Vec<Value>> {
    #[derive(Deserialize, Debug)]
    struct Json {
        entries: Vec<Value>,
    }

    let json: Json = from_str(big_json)?;
    Ok(json.entries)
}
