use std::collections::HashMap;

use jiff::{SignedDuration, Timestamp};
use serenity::all::{
    CommandOptionType,
    CreateAttachment,
    CreateCommandOption,
    EditInteractionResponse,
    ResolvedValue,
    User,
    UserId,
};
use zayden_app::entitlement::Tier;
use zayden_core::{InvocationCtx, as_i64, optional_option, server_tier};
use zayden_graphics::Renderer;

use crate::tree::svg::render;
use crate::tree::{RawGraph, TreeQuota, avatar, compose, cooldown};
use crate::{FamilyError, Result};

const TREE_FILENAME: &str = "family-tree.png";

#[derive(Debug, Clone)]
pub struct TreeImage {
    pub png: Vec<u8>,
    pub target: UserId,
    pub target_name: String,
    pub shown: usize,
    pub total: usize,
    pub tier: Tier,
    pub truncated: bool,
}

impl TreeImage {
    #[must_use]
    pub const fn is_collapsed(&self) -> bool {
        self.shown < self.total
    }
}

pub(super) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "tree",
        "Display a family tree",
    )
    .add_sub_option(super::user_option(
        "The user whose family tree to display. Leave blank for your own",
        false,
    ))
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let target: &User =
        optional_option(&mut options, "user").unwrap_or(&cx.interaction.user);

    let guild_id = cx.interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let tier = server_tier(&cx.ctx.http, &cx.app.entitlements, guild_id).await;

    let image = build(cx, target, tier).await?;

    let file = CreateAttachment::bytes(image.png.clone(), TREE_FILENAME);

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().new_attachment(file),
        )
        .await?;

    Ok(())
}

async fn build(
    cx: &InvocationCtx<'_>,
    target: &User,
    tier: Tier,
) -> Result<TreeImage> {
    let interaction = cx.interaction;
    let pool = &cx.app.db;

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let quota = TreeQuota::for_tier(tier);

    if let Some(left) =
        cooldown::remaining(guild_id, interaction.user.id, quota.cooldown).await
    {
        return Err(FamilyError::TreeCooldown { retry_at: retry_timestamp(left) });
    }

    let raw = RawGraph::fetch(pool, guild_id, target.id, quota).await?;

    if raw.len() < 2 {
        return Err(FamilyError::TreeEmpty(target.id));
    }

    let composed = compose(&raw, as_i64(target.id.get()), quota)
        .ok_or(FamilyError::TreeEmpty(target.id))?;

    let wanted = avatar::selection(&composed.graph);
    let svg = render(&composed.graph, &composed.layout, quota, &wanted);

    let overlays = avatars(&cx.app.http, &svg.avatars, &composed, target).await;

    let png = Renderer::shared()?
        .render(svg.markup, svg.canvas, overlays, quota.raster_limits())
        .await?;

    cooldown::record(guild_id, interaction.user.id).await;

    Ok(TreeImage {
        png,
        target: target.id,
        target_name: target.display_name().to_string(),
        shown: composed.shown,
        total: composed.total,
        tier,
        truncated: composed.truncated,
    })
}

async fn avatars(
    http: &reqwest::Client,
    slots: &[crate::tree::AvatarSlot],
    composed: &crate::tree::Composed,
    target: &User,
) -> Vec<zayden_graphics::Overlay> {
    if slots.is_empty() {
        return Vec::new();
    }

    let target_id = as_i64(target.id.get());
    let hashes: Vec<(i64, Option<String>)> = composed
        .graph
        .people
        .iter()
        .map(|person| {
            let hash = if person.id == target_id {
                target.avatar.map(|hash| hash.to_string())
            } else {
                None
            };
            (person.id, hash)
        })
        .collect();

    avatar::fetch(http, slots, &hashes).await
}

fn retry_timestamp(left: SignedDuration) -> i64 {
    Timestamp::now().as_second().saturating_add(left.as_secs())
}
