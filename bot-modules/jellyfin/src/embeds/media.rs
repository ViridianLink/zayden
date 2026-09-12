use serenity::all::CreateEmbed;

use crate::discovery::Resolved;
use crate::embeds::COLOUR;
use crate::transport::JellyfinClient;
use crate::transport::jellyseerr::model::{MediaInfo, SearchResult};

const POSTER_ROOT: &str = "https://image.tmdb.org/t/p/w342";
const MAX_OVERVIEW: usize = 400;

#[must_use]
pub fn result_line(result: &SearchResult) -> String {
    let on_server = result.media_info.as_ref().is_some_and(MediaInfo::is_available);

    let marker = if on_server { " — **on the server**" } else { "" };

    result.year().map_or_else(
        || format!("**{}**{marker}", result.display_title()),
        |year| format!("**{}** ({year}){marker}", result.display_title()),
    )
}

pub fn list(
    title: &str,
    results: &[SearchResult],
    note: Option<&str>,
) -> CreateEmbed<'static> {
    let mut body =
        results.iter().take(8).map(result_line).collect::<Vec<_>>().join("\n");

    if body.is_empty() {
        "Nothing matched.".clone_into(&mut body);
    }

    if let Some(note) = note {
        body.push_str("\n\n");
        body.push_str(note);
    }

    CreateEmbed::new().title(title.to_owned()).colour(COLOUR).description(body)
}

pub fn resolved(
    client: &JellyfinClient,
    resolved: &Resolved,
) -> CreateEmbed<'static> {
    let mut embed = CreateEmbed::new().title(resolved.title()).colour(COLOUR);

    match resolved {
        Resolved::OnServer { local, remote } => {
            embed = embed.description(format!(
                "Already on the server — [open it]({})",
                client.item_url(&local.item_id)
            ));

            if let Some(remote) = remote
                && let Some(overview) = remote.overview.as_deref()
            {
                embed = embed.field("Overview", truncate(overview), false);
            }
            if let Some(poster) =
                remote.as_ref().and_then(|r| r.poster_path.as_deref())
            {
                embed = embed.thumbnail(format!("{POSTER_ROOT}{poster}"), None);
            }
        },
        Resolved::Remote(remote) => {
            if let Some(overview) = remote.overview.as_deref() {
                embed = embed.description(truncate(overview));
            }
            if let Some(poster) = remote.poster_path.as_deref() {
                embed = embed.thumbnail(format!("{POSTER_ROOT}{poster}"), None);
            }
        },
    }

    embed
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_OVERVIEW {
        return text.to_owned();
    }

    let mut out: String = text.chars().take(MAX_OVERVIEW).collect();
    out.push('…');
    out
}
