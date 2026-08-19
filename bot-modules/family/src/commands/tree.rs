use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    User,
    UserId,
};
use sqlx::PgPool;
use zayden_app::entitlement::Tier;
use zayden_core::{as_i64, optional_option, parse_options};
use zayden_graphics::Renderer;

use crate::tree::svg::render;
use crate::tree::{RawGraph, TreeQuota, avatar, compose, cooldown};
use crate::{FamilyError, Result};

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

pub struct Tree;

impl Tree {
    pub async fn run(
        interaction: &CommandInteraction,
        pool: &PgPool,
        http: &reqwest::Client,
        tier: Tier,
    ) -> Result<TreeImage> {
        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let options = interaction.data.options();
        let mut options = parse_options(options);
        let target: &User =
            optional_option(&mut options, "user").unwrap_or(&interaction.user);

        let quota = TreeQuota::for_tier(tier);

        if let Some(left) =
            cooldown::remaining(guild_id, interaction.user.id, quota.cooldown).await
        {
            return Err(FamilyError::TreeCooldown {
                retry_at: retry_timestamp(left),
            });
        }

        let raw = RawGraph::fetch(pool, guild_id, target.id, quota).await?;

        if raw.len() < 2 {
            return Err(FamilyError::TreeEmpty(target.id));
        }

        let composed = compose(&raw, as_i64(target.id.get()), quota)
            .ok_or(FamilyError::TreeEmpty(target.id))?;

        let wanted = avatar::selection(&composed.graph, quota);
        let svg = render(&composed.graph, &composed.layout, quota, &wanted);

        let overlays = avatars(http, &svg.avatars, &composed, target).await;

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

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("tree")
            .description("Display your family tree.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user whose family tree to display.",
            ))
    }
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

fn retry_timestamp(left: std::time::Duration) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));

    now.saturating_add(i64::try_from(left.as_secs()).unwrap_or(0))
}
