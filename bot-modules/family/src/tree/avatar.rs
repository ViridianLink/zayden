use std::collections::BTreeSet;

use futures::StreamExt;
use serenity::all::UserId;
use tracing::warn;
use zayden_graphics::{AVATAR_MAX_BYTES, Overlay, decode_avatar};

use crate::tree::AVATAR_FETCH_CONCURRENCY;
use crate::tree::model::{FamilyGraph, NodeIdx};
use crate::tree::svg::AvatarSlot;

#[must_use]
pub fn selection(graph: &FamilyGraph) -> BTreeSet<NodeIdx> {
    if graph.is_empty() {
        return BTreeSet::new();
    }

    BTreeSet::from([graph.focus])
}

fn avatar_url(user_id: UserId, hash: Option<&str>, size: u32) -> String {
    hash.map_or_else(
        || {
            let index = (user_id.get() >> 22) % 6;
            format!("https://cdn.discordapp.com/embed/avatars/{index}.png")
        },
        |hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{user_id}/{hash}.png?size={size}"
            )
        },
    )
}

async fn one(
    http: &reqwest::Client,
    slot: AvatarSlot,
    hash: Option<String>,
) -> Option<Overlay> {
    let user_id = UserId::new(u64::try_from(slot.id).ok()?);
    let requested = slot.size.next_power_of_two().clamp(16, 256);
    let url = avatar_url(user_id, hash.as_deref(), requested);

    let response = http.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        warn!(status = %response.status(), %user_id, "avatar fetch failed");
        return None;
    }

    if response.content_length().is_some_and(|len| len > AVATAR_MAX_BYTES as u64) {
        warn!(%user_id, "avatar exceeds the byte cap");
        return None;
    }

    let bytes = response.bytes().await.ok()?;

    match decode_avatar(&bytes, slot.size) {
        Ok(pixmap) => Some(Overlay { pixmap, x: slot.x, y: slot.y }),
        Err(e) => {
            warn!(error = %e, %user_id, "avatar decode failed");
            None
        },
    }
}

pub async fn fetch(
    http: &reqwest::Client,
    slots: &[AvatarSlot],
    hashes: &[(i64, Option<String>)],
) -> Vec<Overlay> {
    futures::stream::iter(slots.iter().copied().map(|slot| {
        let hash = hashes
            .iter()
            .find(|(id, _)| *id == slot.id)
            .and_then(|(_, hash)| hash.clone());

        async move { one(http, slot, hash).await }
    }))
    .buffer_unordered(AVATAR_FETCH_CONCURRENCY)
    .filter_map(|overlay| async move { overlay })
    .collect()
    .await
}
