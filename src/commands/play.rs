use log::instrument;
use poise::{futures_util::TryFutureExt, AutocompleteChoice};

// use url::Url;
use crate::{bot, log, Context, Error, Result};

/// Plays from the given link or does a youtube search on the query.
///
/// This won't do a search until the query is at least 2 characters long.
#[instrument(err, skip(ctx))]
#[poise::command(slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Youtube query or url"]
    #[autocomplete = "autocomplete_query"]
    // query: Url,
    query: String,
) -> Result<()> {
    let call = bot::voice::join_author(&ctx).await?; // Join the call if not in there
    let reply = ctx.say(format!("Playing: {query}")).map_err(|e| e.into());
    let queque_audio = async {
        let audio_source = songbird::ytdl(&query).await?;
        call.lock().await.enqueue_source(audio_source);
        Ok::<(), Error>(())
    };
    // Concurrently send reply & queue song.
    let (_reply_handle, _) = tokio::try_join!(reply, queque_audio)?;
    Ok(())
}

#[instrument(skip(_ctx))]
async fn autocomplete_query(_ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice<String>> {
    // async fn autocomplete_query(_ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice<Url>> {

    let min_partial_len = 2; // Min length before doing actual searches

    if partial.len() <= min_partial_len {
        log::trace!(
            "Skipping search, query length ({}) less than min ({min_partial_len}).",
            partial.len()
        );
        return Vec::new();
    };

    log::trace!("Searching for '{partial}'.");

    let res = bot::youtube::search(partial, 5).await;
    match res {
        Ok(search_res) => search_res
            .into_iter()
            .map(|(name, url)| AutocompleteChoice {
                name,
                value: url.into(),
            })
            .collect(),
        Err(e) => {
            log::error!("{e}");
            Vec::new()
        }
    }
}
