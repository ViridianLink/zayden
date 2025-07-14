use std::cmp;

use async_trait::async_trait;
use chrono::{Months, TimeDelta, Utc};
use core::{SlashCommand, parse_options};
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateEmbed, CreateMessage, EditInteractionResponse,
    Message, Ready, ResolvedOption, ResolvedValue, User, UserId,
};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::prelude::GuildId;
use serenity::model::{Permissions, Timestamp};
use serenity::prelude::Context;
use sqlx::{PgPool, Pool, Postgres};

use crate::{Error, Result};

use super::{InfractionKind, InfractionRow};

pub struct Infraction;

#[async_trait]
impl SlashCommand<Error, Postgres> for Infraction {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(ctx).await.unwrap();

        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::User(user, _)) = options.remove("user") else {
            unreachable!("User option is required");
        };

        let points = match options.remove("points") {
            Some(ResolvedValue::Integer(points)) => points as i32,
            _ => 1,
        };

        let reason = match options.remove("reason") {
            Some(ResolvedValue::String(reason)) => reason,
            _ => "No reason provided.",
        };

        let infractions = InfractionRow::user_infractions(pool, user.id, false).await?;

        let six_months_age = Utc::now()
            .checked_sub_months(Months::new(6))
            .unwrap()
            .naive_utc();

        let infractions: Vec<_> = infractions
            .iter()
            .filter(|infraction| infraction.created_at >= six_months_age)
            .collect();
        let infraction_count = infractions
            .into_iter()
            .map(|infraction| infraction.points)
            .sum::<i32>();
        let infraction_count = cmp::min(infraction_count + points, 5);

        let embed = match infraction_count {
            1 => warn(ctx, pool, guild_id, user, &interaction.user, points, reason).await?,
            2 => {
                mute(
                    ctx,
                    pool,
                    guild_id,
                    user,
                    &interaction.user,
                    TimeDelta::hours(1),
                    points,
                    reason,
                )
                .await?
            }
            3 => {
                mute(
                    ctx,
                    pool,
                    guild_id,
                    user,
                    &interaction.user,
                    TimeDelta::hours(8),
                    points,
                    reason,
                )
                .await?
            }
            4 => {
                mute(
                    ctx,
                    pool,
                    guild_id,
                    user,
                    &interaction.user,
                    TimeDelta::days(28),
                    points,
                    reason,
                )
                .await?
            }
            5 => ban(ctx, pool, guild_id, user, &interaction.user, points, reason).await?,
            _ => unreachable!("Invalid infraction count"),
        };

        interaction
            .edit_response(ctx, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        let command = CreateCommand::new("infraction")
            .name("infraction")
            .description("Warn, mute, or ban a user")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user to warn, mute, or ban",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "points",
                "The number of infractions to give the user",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "The reason for the infraction",
            ));

        Ok(command)
    }
}

async fn send_user_message(
    ctx: &Context,
    user_id: UserId,
    kind: InfractionKind,
    desc: impl Into<String>,
) -> Result<Message> {
    let title = match kind {
        InfractionKind::Warn => "You have been warned",
        InfractionKind::Mute => "You have been muted",
        InfractionKind::Kick => "You have been kicked",
        InfractionKind::SoftBan => "You have been softbanned",
        InfractionKind::Ban => "You have been banned",
    };

    let embed = CreateEmbed::new().title(title).description(desc);

    let message = user_id
        .direct_message(ctx, CreateMessage::new().embed(embed))
        .await
        .unwrap();

    Ok(message)
}

async fn warn<'a>(
    ctx: &Context,
    pool: &PgPool,
    guild_id: GuildId,
    user: &User,
    moderator: &User,
    points: i32,
    reason: &str,
) -> Result<CreateEmbed> {
    InfractionRow::new(
        user.id,
        &user.name,
        guild_id,
        InfractionKind::Warn,
        moderator,
        points,
        reason,
    )?
    .create(pool)
    .await?;

    let desc = if reason == "No reason provided." {
        format!(
            "You have been warned in {}.",
            guild_id.to_partial_guild(ctx).await.unwrap().name
        )
    } else {
        format!(
            "You have been warned in {} for the following reason:\n{}",
            guild_id.to_partial_guild(ctx).await.unwrap().name,
            reason
        )
    };

    send_user_message(ctx, user.id, InfractionKind::Warn, desc).await?;

    let mut embed = CreateEmbed::new().title(format!("{} has been warned", user.name));
    if reason != "No reason provided." {
        embed = embed.description(reason);
    }
    Ok(embed)
}

#[allow(clippy::too_many_arguments)]
async fn mute<'a>(
    ctx: &Context,
    pool: PgPool,
    guild_id: GuildId,
    user: &User,
    moderator: &User,
    duration: TimeDelta,
    points: i32,
    reason: &str,
) -> Result<CreateEmbed> {
    let mut member = guild_id.member(ctx, user.id).await.unwrap();

    let timestamp = (Utc::now() + duration).timestamp();
    member
        .disable_communication_until_datetime(
            ctx,
            Timestamp::from_unix_timestamp(timestamp).unwrap(),
        )
        .await
        .unwrap();

    InfractionRow::new(
        user.id,
        &user.name,
        guild_id,
        InfractionKind::Ban,
        moderator,
        points,
        reason,
    )?
    .create(pool)
    .await?;

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;

    let mut duration_str = String::new();
    if days > 0 {
        duration_str.push_str(&format!("{} day", days));
        if days > 1 {
            duration_str.push('s');
        }
    } else if hours > 0 {
        duration_str.push_str(&format!("{} hour", hours));
        if hours > 1 {
            duration_str.push('s');
        }
    }

    let desc: String = if reason == "No reason provided." {
        format!(
            "You have been muted in {} for {}.",
            guild_id.to_partial_guild(ctx).await.unwrap().name,
            duration_str
        )
    } else {
        format!(
            "You have been muted in {} for {}\n{}",
            guild_id.to_partial_guild(ctx).await.unwrap().name,
            duration_str,
            reason
        )
    };

    send_user_message(ctx, user.id, InfractionKind::Mute, &desc).await?;

    let mut embed = CreateEmbed::new().title(format!("{} has been muted", user.name));
    if reason != "No reason provided." {
        embed = embed.description(reason);
    }
    Ok(embed)
}

async fn ban<'a>(
    ctx: &Context,
    pool: PgPool,
    guild_id: GuildId,
    user: &User,
    moderator: &User,
    points: i32,
    reason: &str,
) -> Result<CreateEmbed> {
    let member = guild_id.member(ctx, user.id).await.unwrap();

    let desc = if reason == "No reason provided." {
        format!(
            "You have been banned from {}.",
            guild_id.to_partial_guild(ctx).await.unwrap().name
        )
    } else {
        format!(
            "You have been banned from {} for the following reason:\n{}",
            guild_id.to_partial_guild(ctx).await.unwrap().name,
            reason
        )
    };

    send_user_message(ctx, user.id, InfractionKind::Ban, desc)
        .await
        .unwrap();

    member.ban_with_reason(ctx, 1, reason).await.unwrap();

    InfractionRow::new(
        user.id,
        &user.name,
        guild_id,
        InfractionKind::Ban,
        moderator,
        points,
        reason,
    )?
    .create(pool)
    .await?;

    let mut embed = CreateEmbed::new().title(format!("{} has been banned", user.name));
    if reason != "No reason provided." {
        embed = embed.description(reason);
    }
    Ok(embed)
}
