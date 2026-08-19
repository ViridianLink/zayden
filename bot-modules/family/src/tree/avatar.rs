use std::collections::BTreeSet;

use futures::StreamExt;
use serenity::all::UserId;
use tracing::warn;
use zayden_graphics::{AVATAR_MAX_BYTES, Overlay, decode_avatar};

use crate::tree::model::{FamilyGraph, NodeIdx};
use crate::tree::svg::AvatarSlot;
use crate::tree::{AVATAR_FETCH_CONCURRENCY, TreeQuota};

#[must_use]
pub fn selection(graph: &FamilyGraph, quota: TreeQuota) -> BTreeSet<NodeIdx> {
    if quota.avatars == 0 || graph.is_empty() {
        return BTreeSet::new();
    }

    let mut ordered: Vec<NodeIdx> = vec![graph.focus];

    let push = |node: NodeIdx, ordered: &mut Vec<NodeIdx>| {
        if !ordered.contains(&node) {
            ordered.push(node);
        }
    };

    for &(a, b) in &graph.partner_edges {
        if a == graph.focus {
            push(b, &mut ordered);
        } else if b == graph.focus {
            push(a, &mut ordered);
        }
    }

    for &(parent, child) in &graph.parent_edges {
        if child == graph.focus {
            push(parent, &mut ordered);
        }
    }
    for &(parent, child) in &graph.parent_edges {
        if parent == graph.focus {
            push(child, &mut ordered);
        }
    }

    for node in 0..graph.len() {
        push(node, &mut ordered);
    }

    ordered.into_iter().take(quota.avatars).collect()
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
