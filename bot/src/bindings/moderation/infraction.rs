use std::time::Duration;

use async_trait::async_trait;
use serenity::all::{
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateMessage,
    EditInteractionResponse,
    GuildId,
    Permissions,
    Timestamp,
    User,
    UserId,
};
use sqlx::PgPool;
use zayden_core::error::CoreError;
use zayden_core::{
    HandlerError,
    InvocationCtx,
    ModuleCommand,
    as_i64,
    optional_option,
    parse_options,
    required_option,
};

use super::{InfractionKind, InfractionRow, NO_REASON};

pub(super) struct Infraction;

#[async_trait]
impl ModuleCommand for Infraction {
    fn module(&self) -> Option<&'static str> {
        Some("moderation")
    }

    fn definition(&self) -> CreateCommand<'static> {
        CreateCommand::new("infraction")
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
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "points",
                    "The number of infractions to give the user",
                )
                .min_int_value(1),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "The reason for the infraction",
            ))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let guild_id = cx.interaction.guild_id.ok_or(CoreError::MissingGuildId)?;

        let options = cx.interaction.data.options();
        let mut options = parse_options(options);

        let user: &User = required_option(&mut options, "user")?;
        let points = optional_option::<i64, _>(&mut options, "points")
            .unwrap_or(1)
            .try_into()
            .unwrap_or(i32::MAX);
        let reason =
            optional_option::<&str, _>(&mut options, "reason").unwrap_or(NO_REASON);

        let recent =
            InfractionRow::user_infractions(&cx.app.db, user.id, true).await?;
        let prior_points: i32 = recent.iter().map(|row| row.points).sum();

        let infraction_count = prior_points.saturating_add(points).clamp(1, 5);

        let case = Case {
            ctx: cx.ctx,
            pool: &cx.app.db,
            guild_id,
            target: user,
            moderator: &cx.interaction.user,
            points,
            reason,
        };

        let embed = match infraction_count {
            ..=1 => warn(&case).await?,
            2 => mute(&case, Duration::from_hours(1)).await?,
            3 => mute(&case, Duration::from_hours(8)).await?,
            4 => mute(&case, Duration::from_hours(28 * 24)).await?,
            _ => ban(&case).await?,
        };

        cx.interaction
            .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}

struct Case<'a> {
    ctx: &'a Context,
    pool: &'a PgPool,
    guild_id: GuildId,
    target: &'a User,
    moderator: &'a User,
    points: i32,
    reason: &'a str,
}

impl Case<'_> {
    async fn record(&self, kind: InfractionKind) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO infractions
                (user_id, username, guild_id, infraction_type,
                 moderator_id, moderator_username, points, reason)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            as_i64(self.target.id.get()),
            self.target.name.as_str(),
            as_i64(self.guild_id.get()),
            kind as _,
            as_i64(self.moderator.id.get()),
            self.moderator.name.as_str(),
            self.points,
            self.reason,
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }
}

async fn warn(case: &Case<'_>) -> Result<CreateEmbed<'static>, HandlerError> {
    case.record(InfractionKind::Warn).await?;

    let guild_name = case.guild_id.to_partial_guild(&case.ctx.http).await?.name;
    let desc = if case.reason == NO_REASON {
        format!("You have been warned in {guild_name}.")
    } else {
        format!(
            "You have been warned in {guild_name} for the following reason:\n{}",
            case.reason
        )
    };

    let _ = send_user_message(case.ctx, case.target.id, InfractionKind::Warn, desc)
        .await;

    Ok(action_embed(case.target, "warned", case.reason))
}

async fn mute(
    case: &Case<'_>,
    duration: Duration,
) -> Result<CreateEmbed<'static>, HandlerError> {
    let mut member = case.guild_id.member(&case.ctx.http, case.target.id).await?;

    let until = Timestamp::from_unix_timestamp(
        Timestamp::now()
            .unix_timestamp()
            .saturating_add(i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)),
    )
    .map_err(|e| CoreError::Other(e.to_string()))?;
    member.disable_communication_until(&case.ctx.http, until).await?;

    case.record(InfractionKind::Mute).await?;

    let guild_name = case.guild_id.to_partial_guild(&case.ctx.http).await?.name;
    let duration_str = format_duration(duration);
    let desc = if case.reason == NO_REASON {
        format!("You have been muted in {guild_name} for {duration_str}.")
    } else {
        format!(
            "You have been muted in {guild_name} for {duration_str}\n{}",
            case.reason
        )
    };

    let _ = send_user_message(case.ctx, case.target.id, InfractionKind::Mute, desc)
        .await;

    Ok(action_embed(case.target, "muted", case.reason))
}

async fn ban(case: &Case<'_>) -> Result<CreateEmbed<'static>, HandlerError> {
    let member = case.guild_id.member(&case.ctx.http, case.target.id).await?;

    let guild_name = case.guild_id.to_partial_guild(&case.ctx.http).await?.name;
    let desc = if case.reason == NO_REASON {
        format!("You have been banned from {guild_name}.")
    } else {
        format!(
            "You have been banned from {guild_name} for the following reason:\n{}",
            case.reason
        )
    };

    let _ =
        send_user_message(case.ctx, case.target.id, InfractionKind::Ban, desc).await;

    member.ban(&case.ctx.http, 1, Some(case.reason)).await?;

    case.record(InfractionKind::Ban).await?;

    Ok(action_embed(case.target, "banned", case.reason))
}

async fn send_user_message(
    ctx: &Context,
    user_id: UserId,
    kind: InfractionKind,
    desc: impl Into<String>,
) -> Result<(), HandlerError> {
    let title = match kind {
        InfractionKind::Warn => "You have been warned",
        InfractionKind::Mute => "You have been muted",
        InfractionKind::Kick => "You have been kicked",
        InfractionKind::SoftBan => "You have been softbanned",
        InfractionKind::Ban => "You have been banned",
    };

    let embed = CreateEmbed::new().title(title).description(desc.into());

    user_id.direct_message(&ctx.http, CreateMessage::new().embed(embed)).await?;

    Ok(())
}

fn action_embed(
    target: &User,
    past_tense: &str,
    reason: &str,
) -> CreateEmbed<'static> {
    let mut embed =
        CreateEmbed::new().title(format!("{} has been {past_tense}", target.name));
    if reason != NO_REASON {
        embed = embed.description(reason.to_string());
    }
    embed
}

fn format_duration(duration: Duration) -> String {
    const HOUR: u64 = 60 * 60;
    const DAY: u64 = 24 * HOUR;

    let secs = duration.as_secs();

    let days = secs / DAY;
    if days > 0 {
        return format!("{days} day{}", if days > 1 { "s" } else { "" });
    }

    let hours = secs / HOUR;
    format!("{hours} hour{}", if hours > 1 { "s" } else { "" })
}
