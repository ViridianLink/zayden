use std::fmt::Display;

use serenity::all::UserId;
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::RegistryBuilder;

mod infraction;
mod logs;
mod rules;

use infraction::Infraction;
use logs::Logs;
use rules::RulesCommand;

pub(crate) const NO_REASON: &str = "No reason provided.";

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Infraction).add_command(Logs).add_command(RulesCommand);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "infraction_kind")]
pub(crate) enum InfractionKind {
    Warn,
    Mute,
    Kick,
    SoftBan,
    Ban,
}

impl Display for InfractionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Warn => "Warn",
            Self::Mute => "Mute",
            Self::Kick => "Kick",
            Self::SoftBan => "SoftBan",
            Self::Ban => "Ban",
        };
        f.write_str(s)
    }
}

pub(crate) struct InfractionRow {
    pub id: i32,
    pub user_id: i64,
    pub username: String,
    pub infraction_type: InfractionKind,
    pub moderator_id: i64,
    pub moderator_username: String,
    pub points: i32,
    pub reason: String,
}

impl InfractionRow {
    pub(crate) async fn user_infractions(
        pool: &PgPool,
        user_id: UserId,
        recent: bool,
    ) -> sqlx::Result<Vec<Self>> {
        let user_id = as_i64(user_id.get());

        if recent {
            sqlx::query_as!(
                InfractionRow,
                r#"SELECT
                    id,
                    user_id,
                    username,
                    infraction_type AS "infraction_type: InfractionKind",
                    moderator_id,
                    moderator_username,
                    points,
                    reason
                FROM infractions
                WHERE user_id = $1
                    AND created_at > now() - INTERVAL '6 months'"#,
                user_id
            )
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as!(
                InfractionRow,
                r#"SELECT
                    id,
                    user_id,
                    username,
                    infraction_type AS "infraction_type: InfractionKind",
                    moderator_id,
                    moderator_username,
                    points,
                    reason
                FROM infractions
                WHERE user_id = $1"#,
                user_id
            )
            .fetch_all(pool)
            .await
        }
    }
}
