use serenity::all::{CreateEmbed, CreateEmbedFooter, GuildId, Mentionable};
use sqlx::PgPool;

use crate::{LeaderboardRow, LevelsRow, Result};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LeaderboardScope {
    Guild,
    Global,
}

impl LeaderboardScope {
    #[must_use]
    pub const fn from_global_flag(global: bool) -> Self {
        if global { Self::Global } else { Self::Guild }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Guild => "Server Leaderboard",
            Self::Global => "Global Leaderboard",
        }
    }

    #[must_use]
    pub const fn footer_tag(self) -> &'static str {
        match self {
            Self::Guild => "Server",
            Self::Global => "Global",
        }
    }

    #[must_use]
    pub fn from_footer_tag(tag: &str) -> Self {
        match tag {
            "Global" => Self::Global,
            _ => Self::Guild,
        }
    }
}

pub async fn create_embed<'a>(
    pool: &PgPool,
    guild_id: GuildId,
    scope: LeaderboardScope,
    page_number: i64,
) -> Result<CreateEmbed<'a>> {
    let rows = match scope {
        LeaderboardScope::Guild => {
            LeaderboardRow::guild_leaderboard(pool, guild_id, page_number).await?
        },
        LeaderboardScope::Global => {
            LeaderboardRow::global_leaderboard(pool, page_number).await?
        },
    };

    let desc = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            row_as_desc(
                &row,
                i + (usize::try_from(page_number).unwrap_or(0).saturating_sub(1))
                    * 10,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let embed = CreateEmbed::new().title(scope.title()).description(desc).footer(
        CreateEmbedFooter::new(format!(
            "Page {page_number} · {}",
            scope.footer_tag()
        )),
    );

    Ok(embed)
}

#[must_use]
pub fn row_as_desc(row: &LeaderboardRow, i: usize) -> String {
    let place = if i == 0 {
        "🥇".to_string()
    } else if i == 1 {
        "🥈".to_string()
    } else if i == 2 {
        "🥉".to_string()
    } else {
        format!("#{}", i + 1)
    };

    let data = format!(
        "{}\n(Messages: {} | Total XP: {})",
        row.level(),
        row.message_count(),
        row.xp(),
    );

    format!("{place} - {} - {data}", row.user_id().mention())
}
