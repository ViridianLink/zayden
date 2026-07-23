use std::collections::HashSet;

use serenity::all::{ChannelId, UserId};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use crate::Result;

#[derive(FromRow, Clone)]
pub struct VoiceChannelRow {
    pub id: i64,
    pub owner_id: i64,
    pub trusted_ids: Vec<i64>,
    pub invites: Vec<i64>,
    pub password: Option<String>,
    pub persistent: bool,
    pub mode: VoiceChannelMode,
}

impl VoiceChannelRow {
    #[must_use]
    pub const fn new(id: ChannelId, owner_id: UserId) -> Self {
        Self {
            id: as_i64(id.get()),
            owner_id: as_i64(owner_id.get()),
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: false,
            mode: VoiceChannelMode::Open,
        }
    }

    #[must_use]
    pub const fn new_persistent(id: ChannelId, owner_id: UserId) -> Self {
        Self {
            id: as_i64(id.get()),
            owner_id: as_i64(owner_id.get()),
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: true,
            mode: VoiceChannelMode::Open,
        }
    }

    #[must_use]
    pub const fn channel_id(&self) -> ChannelId {
        ChannelId::new(as_u64(self.id))
    }

    #[must_use]
    pub const fn owner_id(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    #[must_use]
    pub fn trusted_ids(&self) -> HashSet<UserId> {
        self.trusted_ids.iter().map(|id| UserId::new(as_u64(*id))).collect()
    }

    #[must_use]
    pub fn invites(&self) -> HashSet<UserId> {
        self.invites.iter().map(|id| UserId::new(as_u64(*id))).collect()
    }

    #[must_use]
    pub fn is_owner(&self, user_id: UserId) -> bool {
        self.owner_id() == user_id
    }

    pub const fn set_owner(&mut self, id: UserId) {
        self.owner_id = as_i64(id.get());
    }

    #[must_use]
    pub fn is_trusted(&self, user_id: UserId) -> bool {
        self.trusted_ids().contains(&user_id) || self.owner_id() == user_id
    }

    #[must_use]
    pub fn verify_password(&self, pass: &str) -> bool {
        self.password.as_deref() == Some(pass)
    }

    #[must_use]
    pub const fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub const fn toggle_persist(&mut self) {
        self.persistent = !self.persistent;
    }

    pub fn trust(&mut self, id: UserId) {
        self.trusted_ids.push(as_i64(id.get()));
    }

    pub fn untrust(&mut self, id: UserId) {
        let id = as_i64(id.get());

        self.trusted_ids.retain(|trusted_id| *trusted_id != id);
    }

    pub fn create_invite(&mut self, id: UserId) {
        self.invites.push(as_i64(id.get()));
    }

    pub fn block(&mut self, id: UserId) {
        let id = as_i64(id.get());

        self.trusted_ids.retain(|trusted_id| *trusted_id != id);
        self.invites.retain(|invite| *invite != id);
    }

    pub fn reset(&mut self) {
        self.trusted_ids.clear();
        self.invites.clear();
        self.password = None;
    }

    pub async fn get(pool: &PgPool, id: ChannelId) -> sqlx::Result<Option<Self>> {
        let row = sqlx::query_as!(
            VoiceChannelRow,
            r#"SELECT 
                vc.id, 
                vc.owner_id, 
                COALESCE(
                    (SELECT array_agg(user_id) FROM voice_channel_trusted_users WHERE channel_id = vc.id), 
                    ARRAY[]::int[]
                ) AS "trusted_ids!", 
                COALESCE(
                    (SELECT array_agg(user_id) FROM voice_channel_invites WHERE channel_id = vc.id), 
                    ARRAY[]::int[]
                ) AS "invites!", 
                vc.password, 
                vc.persistent, 
                vc.mode AS "mode: TempVoiceMode" 
            FROM voice_channels vc
            WHERE vc.id = $1;"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn count_persistent_channels(
        pool: &PgPool,
        user_id: UserId,
    ) -> sqlx::Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM voice_channels WHERE owner_id = $1 AND persistent = true"#,
            as_i64(user_id.get())
        )
        .fetch_one(pool)
        .await?;

        count.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn claim(
        pool: &PgPool,
        id: ChannelId,
        expected_owner: UserId,
        new_owner: UserId,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "UPDATE voice_channels SET owner_id = $2 WHERE id = $1 AND owner_id = $3",
            as_i64(id.get()),
            as_i64(new_owner.get()),
            as_i64(expected_owner.get()),
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for custom temp_voice_mode pgtype"
    )]
    pub async fn save(self, pool: &PgPool) -> Result<PgQueryResult> {
        let mode = TempVoiceMode::from(self.mode);

        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            r#"
            INSERT INTO voice_channels (id, owner_id, password, persistent, mode)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
            SET owner_id = $2, password = $3, persistent = $4, mode = $5
            "#,
            self.id,
            self.owner_id,
            self.password,
            self.persistent,
            mode as TempVoiceMode
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            r#"
            WITH deleted AS (
                DELETE FROM voice_channel_trusted_users WHERE channel_id = $1
            )
            INSERT INTO voice_channel_trusted_users (channel_id, user_id)
            SELECT $1, * FROM UNNEST($2::bigint[])
            "#,
            self.id,
            &self.trusted_ids
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            r#"
            WITH deleted AS (
                DELETE FROM voice_channel_invites WHERE channel_id = $1
            )
            INSERT INTO voice_channel_invites (channel_id, user_id)
            SELECT $1, * FROM UNNEST($2::bigint[])
            "#,
            self.id,
            &self.invites
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        result.extend([result2, result3]);

        Ok(result)
    }

    pub async fn delete(self, pool: &PgPool) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            r#"DELETE FROM voice_channels WHERE id = $1"#,
            as_i64(self.channel_id().get())
        )
        .execute(pool)
        .await
    }
}

#[derive(sqlx::Type, Clone)]
#[sqlx(rename_all = "lowercase")]
pub enum VoiceChannelMode {
    Open,
    Spectator,
    Locked,
    Invisible,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "temp_voice_mode")]
struct TempVoiceMode(VoiceChannelMode);

impl From<VoiceChannelMode> for TempVoiceMode {
    fn from(mode: VoiceChannelMode) -> Self {
        Self(mode)
    }
}

impl From<TempVoiceMode> for VoiceChannelMode {
    fn from(wrapper: TempVoiceMode) -> Self {
        wrapper.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_persistent_registers_caller_as_owner_of_an_open_channel() {
        let channel_id = ChannelId::new(123);
        let owner_id = UserId::new(456);

        let row = VoiceChannelRow::new_persistent(channel_id, owner_id);

        assert_eq!(row.channel_id(), channel_id);
        assert_eq!(row.owner_id(), owner_id);
        assert!(row.is_owner(owner_id));
        assert!(row.is_persistent());
        assert!(matches!(row.mode, VoiceChannelMode::Open));
        assert!(row.trusted_ids.is_empty());
        assert!(row.invites.is_empty());
        assert!(row.password.is_none());
    }
}
