use std::collections::HashMap;

use serenity::all::ResolvedValue;
use sqlx::PgPool;
use zayden_core::{InvocationCtx, optional_option};

use super::{resolve_player_source, respond};
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::save::uid_to_filename;
use crate::{embeds, progress};

pub(super) const CATEGORIES: &[(&str, &str)] = &[
    ("fast-travel", "Fast travel points"),
    ("towers", "Tower bosses"),
    ("bosses", "Field & alpha bosses"),
    ("bounties", "Bounty targets"),
    ("effigies", "Lifmunk effigies"),
    ("relics", "Stat relics"),
    ("areas", "Map areas"),
    ("tree-fast-travel", "World Tree: fast travel points"),
    ("tree-towers", "World Tree: tower bosses"),
    ("tree-bosses", "World Tree: field & alpha bosses"),
    ("tree-bounties", "World Tree: bounty targets"),
    ("tree-effigies", "World Tree: Lifmunk effigies"),
    ("tree-relics", "World Tree: stat relics"),
    ("paldeck", "Paldeck entries"),
    ("captures", "Species caught"),
    ("captures-5x", "Species caught 5x"),
    ("condensed", "Species fully condensed (4★)"),
    ("technology", "Technology"),
    ("missions", "Missions"),
    ("arena", "Arena ranks"),
];

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let player: Option<&str> = optional_option(&mut options, "player");
    let category: Option<&str> = optional_option(&mut options, "category");

    cx.interaction.defer(&cx.ctx.http).await?;

    let (source, roster) = resolve_player_source(cx, client, pool, player).await?;
    let record = client.player_record(source, &roster.uid).await?;

    if record.is_none() {
        return Err(PalworldError::NoPlayerSave {
            player: roster.name.clone(),
            file: uid_to_filename(&roster.uid).unwrap_or_else(|| "<id>".into()),
        });
    }

    let computed =
        progress::compute(record.as_deref(), &roster, progress::catalogue());

    let component = match category {
        Some(key) => {
            let milestone =
                computed.milestone(key).ok_or_else(|| PalworldError::NotFound {
                    entity: "progress category",
                    query: key.to_string(),
                })?;
            embeds::progress_detail_component(&computed, milestone)
        },
        None => embeds::progress_component(&computed),
    };

    respond(cx, component).await
}
