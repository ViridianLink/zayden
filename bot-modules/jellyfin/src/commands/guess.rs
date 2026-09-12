use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateAttachment,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    EditInteractionResponse,
    ResolvedValue,
};
use zayden_core::{InvocationCtx, optional_option, required_option};

use crate::components::GUESS_OPEN_PREFIX;
use crate::embeds::COLOUR;
use crate::error::{JellyfinError, Result};
use crate::games::question::poster::{self, Difficulty};
use crate::games::question::redact;
use crate::games::round::{NewRound, RoundRow};
use crate::index::LibraryItemRow;
use crate::runtime::JellyfinRuntime;
use crate::transport::jellyseerr::model::MovieDetails;

pub const GAME: &str = "guess";

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let mode: &str = required_option(&mut options, "mode")?;
    let difficulty = Difficulty::parse(
        optional_option(&mut options, "difficulty").unwrap_or("easy"),
    );

    let guild_id = cx.interaction.guild_id.ok_or(JellyfinError::MissingGuildId)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let candidate = pick(cx, runtime).await?;
    let tmdb_id = candidate
        .tmdb_id
        .ok_or_else(|| JellyfinError::NoSuchTitle(candidate.name.clone()))?;

    let details = runtime.seer.movie(tmdb_id).await.map_err(JellyfinError::from)?;

    let poster_url = details.poster_path.as_deref().map(poster::poster_url);

    let response = if mode == "visual" {
        let url = poster_url
            .as_deref()
            .ok_or_else(|| JellyfinError::NoSuchTitle(details.title.clone()))?;

        visual(&cx.app.http, url, difficulty, tmdb_id).await?
    } else {
        text(&details, &candidate.name)
    };

    let round_id = NewRound {
        guild_id,
        channel_id: cx.interaction.channel_id,
        started_by: cx.interaction.user.id,
        started_by_name: &cx.interaction.user.name,
        game: GAME,
        kind: mode,
        answer: &candidate.name,
        choices: None,
        reveal_image_url: poster_url.as_deref(),
    }
    .open(&cx.app.db)
    .await?;

    let open = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new(format!("{GUESS_OPEN_PREFIX}{round_id}"))
            .label("Guess")
            .style(ButtonStyle::Primary),
    ]));

    let message = cx
        .interaction
        .edit_response(&cx.ctx.http, response.components(vec![open]))
        .await?;

    RoundRow::set_message(&cx.app.db, round_id, message.id).await?;

    Ok(())
}

async fn pick(
    cx: &InvocationCtx<'_>,
    _runtime: &Arc<JellyfinRuntime>,
) -> Result<LibraryItemRow> {
    let pool = &cx.app.db;

    let row = sqlx::query_scalar!(
        r#"
        SELECT item_id
        FROM jellyfin_library_items
        WHERE item_type = 'Movie' AND tmdb_id IS NOT NULL
        ORDER BY random()
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    let item_id = row.ok_or(JellyfinError::EmptyIndex)?;

    LibraryItemRow::by_id(pool, &item_id).await?.ok_or(JellyfinError::EmptyIndex)
}

async fn visual(
    http: &reqwest::Client,
    poster_url: &str,
    difficulty: Difficulty,
    tmdb_id: i32,
) -> Result<EditInteractionResponse<'static>> {
    let downloaded = http
        .get(poster_url)
        .send()
        .await
        .map_err(|e| JellyfinError::Internal(format!("poster fetch failed: {e}")))?
        .bytes()
        .await
        .map_err(|e| JellyfinError::Internal(format!("poster read failed: {e}")))?;

    let derived = poster::derive(&downloaded, difficulty)?;
    let name = format!("guess-{tmdb_id}.png");

    Ok(EditInteractionResponse::new()
        .embed(
            CreateEmbed::new()
                .title("Guess the film")
                .colour(COLOUR)
                .description(difficulty.hint())
                .attachment(name.clone()),
        )
        .new_attachment(CreateAttachment::bytes(derived, name)))
}

fn text(details: &MovieDetails, title: &str) -> EditInteractionResponse<'static> {
    let overview = details.overview.as_deref().unwrap_or_default();

    EditInteractionResponse::new().embed(
        CreateEmbed::new()
            .title("Guess the film")
            .colour(COLOUR)
            .description(redact::redact(overview, title)),
    )
}
