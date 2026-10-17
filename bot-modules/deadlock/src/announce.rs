use futures::StreamExt;
use serenity::all::{
    CreateComponent,
    CreateMessage,
    CreateTextDisplay,
    Http,
    MessageFlags,
};
use sqlx::PgPool;
use tracing::error;
use zayden_core::announce::{allowed_roles, escape, render};
use zayden_core::limits::message::COMPONENTS_TEXT as MAX_TEXT;

use crate::db::announce::{self, PatchAnnounceRow};
use crate::db::patches::{self, PatchRow};
use crate::embeds;
use crate::error::Result;

const CONCURRENCY: usize = 5;

pub const DEFAULT_MESSAGE: &str = "{roles} New Deadlock patch: **{title}**";

pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("{roles}", "the roles to ping"),
    ("{title}", "the patch's title"),
    ("{link}", "the link to the official patch notes"),
];

pub fn patch_message(
    row: &PatchAnnounceRow,
    patch: &PatchRow,
) -> CreateMessage<'static> {
    let title = escape(&patch.title);
    let content = render(
        row.message.as_deref().unwrap_or(DEFAULT_MESSAGE),
        &row.ping_role_ids,
        &[("title", &title), ("link", &patch.link)],
    );

    let card_budget = MAX_TEXT.saturating_sub(content.chars().count());
    let card = embeds::patch_component(patch, card_budget);

    let components = (!content.is_empty())
        .then(|| CreateComponent::TextDisplay(CreateTextDisplay::new(content)))
        .into_iter()
        .chain([card])
        .collect::<Vec<_>>();

    CreateMessage::new()
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(components)
        .allowed_mentions(allowed_roles(&row.ping_role_ids))
}

pub async fn announce_ready(
    http: &Http,
    pool: &PgPool,
    application_id: i64,
) -> Result<usize> {
    let ready = patches::ready(pool).await?;
    if ready.is_empty() {
        return Ok(0);
    }

    let targets = announce::targets(pool, application_id).await?;

    for patch in &ready {
        let posts: Vec<_> =
            targets.iter().map(|row| post(http, row, patch)).collect();

        futures::stream::iter(posts)
            .buffer_unordered(CONCURRENCY)
            .for_each(|()| async {})
            .await;

        patches::mark_announced(pool, &patch.guid).await?;
    }

    Ok(ready.len())
}

async fn post(http: &Http, row: &PatchAnnounceRow, patch: &PatchRow) {
    let message = patch_message(row, patch);

    if let Err(e) = row.channel().widen().send_message(http, message).await {
        error!(
            error = ?e,
            guild_id = row.guild_id,
            "deadlock: failed to post patch announcement; it is not retried"
        );
    }
}
