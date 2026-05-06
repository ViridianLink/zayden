use std::fmt::Display;

use chrono::NaiveDateTime;
use serenity::all::{Context, CreateCommand, GuildId, Ready, User, UserId};
use sqlx::{postgres::PgQueryResult, FromRow, PgPool};
use core::SlashCommand;

pub use infraction::Infraction;
pub use logs::Logs;
pub use rules::RulesCommand;

use crate::Result;

mod infraction;
mod infraction_kind;
mod infraction_row;
mod logs;
mod rules;

pub fn register(ctx: &Context, ready: &Ready) -> Result<Vec<CreateCommand>> {
    let commands = vec![
        Infraction::register(ctx, ready)?,
        Logs::register(ctx, ready)?,
        RulesCommand::register(ctx, ready)?,
    ];

    Ok(commands)
}

#[derive(FromRow)]
pub struct InfractionRow {
    pub id: i32,
    pub user_id: i64,
    pub username: String,
    pub guild_id: i64,
    pub infraction_type: String,
    pub moderator_id: i64,
    pub moderator_username: String,
    pub points: i32,
    pub reason: String,
    pub created_at: NaiveDateTime,
}

impl InfractionRow {
    fn new(
        user_id: UserId,
        username: impl Into<String>,
        guild_id: GuildId,
        infraction_kind: InfractionKind,
        moderator: &User,
        points: i32,
        reason: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            id: 0,
            user_id: user_id.get() as i64,
            username: username.into(),
            guild_id: guild_id.get() as i64,
            infraction_type: infraction_kind.to_string(),
            moderator_id: moderator.id.get() as i64,
            moderator_username: moderator.name.clone(),
            points,
            reason: reason.into(),
            created_at: chrono::Utc::now().naive_utc(),
        })
    }

    async fn user_infractions(
        pool: &PgPool,
        user_id: UserId,
        recent: bool,
    ) -> Result<Vec<InfractionRow>> {
        let user_id = user_id.get() as i64;

        let infractions = if recent {
            sqlx::query_as!(
                InfractionRow,
                "SELECT * FROM infractions WHERE user_id = $1 AND created_at > CURRENT_DATE - INTERVAL '6 months'",
                user_id
            ).fetch_all(pool).await.unwrap()
        } else {
            sqlx::query_as!(
                InfractionRow,
                "SELECT * FROM infractions WHERE user_id = $1",
                user_id
            )
            .fetch_all(pool)
            .await
            .unwrap()
        };

        Ok(infractions)
    }

    async fn create(&self, pool: &PgPool) -> Result<PgQueryResult> {
        let result = sqlx::query!(
            "INSERT INTO infractions (user_id, username, guild_id, infraction_type, moderator_id, moderator_username, points, reason) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            self.user_id, self.username, self.guild_id, self.infraction_type, self.moderator_id, self.moderator_username, self.points, self.reason
        )
            .execute(pool)
            .await.unwrap();

        Ok(result)
    }
}

#[allow(dead_code)]
enum InfractionKind {
    Warn,
    Mute,
    Kick,
    SoftBan,
    Ban,
}

impl Display for InfractionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            InfractionKind::Warn => "Warn",
            InfractionKind::Mute => "Mute",
            InfractionKind::Kick => "Kick",
            InfractionKind::SoftBan => "SoftBan",
            InfractionKind::Ban => "Ban",
        };
        write!(f, "{}", str)
    }
}
