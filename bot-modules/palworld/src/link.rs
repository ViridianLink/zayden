use sqlx::PgPool;

use crate::error::Result;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlayerLink {
    pub discord_id: i64,
    pub player_uid: String,
    pub in_game_name: String,
}

impl PlayerLink {
    pub async fn select(pool: &PgPool, discord_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            "SELECT discord_id, player_uid, in_game_name FROM palworld_player_links WHERE discord_id = $1",
            discord_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert(
        pool: &PgPool,
        discord_id: i64,
        player_uid: &str,
        in_game_name: &str,
    ) -> Result<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"
            INSERT INTO palworld_player_links (discord_id, player_uid, in_game_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (discord_id) DO UPDATE
                SET player_uid = EXCLUDED.player_uid,
                    in_game_name = EXCLUDED.in_game_name,
                    linked_at = now()
            RETURNING discord_id, player_uid, in_game_name
            "#,
            discord_id,
            player_uid,
            in_game_name
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn delete(pool: &PgPool, discord_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM palworld_player_links WHERE discord_id = $1",
            discord_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
