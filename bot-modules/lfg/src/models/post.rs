use jiff::Zoned;
use jiff::tz::TimeZone;
use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::{GenericChannelId, MessageId, ThreadId, UserId};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use crate::templates::TemplateInfo;
use crate::{Join, LfgError, Result};

pub struct PostBuilder {
    id: ThreadId,
    owner: UserId,
    activity: String,
    start_time: Zoned,
    description: String,
    fireteam_size: i16,
    fireteam: Vec<UserId>,
    alternatives: Vec<UserId>,
    schedule_channel: Option<GenericChannelId>,
    alt_message: Option<MessageId>,
}

impl PostBuilder {
    pub fn new(
        owner: UserId,
        activity: impl Into<String>,
        start: Zoned,
        desc: impl Into<String>,
        fireteam_size: i16,
    ) -> Self {
        Self {
            id: ThreadId::default(),
            owner,
            activity: activity.into(),
            start_time: start,
            description: desc.into(),
            fireteam_size,
            fireteam: vec![owner],
            alternatives: Vec::new(),
            schedule_channel: None,
            alt_message: None,
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<ThreadId>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn activity(mut self, activity: impl Into<String>) -> Self {
        self.activity = activity.into();
        self
    }

    #[must_use]
    pub const fn fireteam_size(mut self, size: i16) -> Self {
        self.fireteam_size = size;
        self
    }

    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    #[must_use]
    pub fn start(mut self, start: Zoned) -> Self {
        self.start_time = start;
        self
    }

    #[must_use]
    pub const fn schedule_channel(mut self, channel: GenericChannelId) -> Self {
        self.schedule_channel = Some(channel);
        self
    }

    #[must_use]
    pub const fn alt_message(mut self, message: MessageId) -> Self {
        self.alt_message = Some(message);
        self
    }

    #[must_use]
    pub fn build(self) -> PostRow {
        PostRow {
            id: as_i64(self.id.get()),
            owner_id: as_i64(self.owner.get()),
            activity: self.activity,
            start_time: self.start_time.timestamp().to_sqlx(),
            description: self.description,
            fireteam_size: self.fireteam_size,
            fireteam: self
                .fireteam
                .into_iter()
                .map(|user| as_i64(user.get()))
                .collect(),
            alternatives: self
                .alternatives
                .into_iter()
                .map(|user| as_i64(user.get()))
                .collect(),
            alt_channel: self.schedule_channel.map(|channel| as_i64(channel.get())),
            alt_message: self.alt_message.map(|message| as_i64(message.get())),
        }
    }
}

impl TemplateInfo for PostBuilder {
    fn activity(&self) -> &str {
        &self.activity
    }

    fn timestamp(&self) -> jiff::Timestamp {
        self.start_time.timestamp()
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn fireteam_size(&self) -> i16 {
        self.fireteam_size
    }

    fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().copied()
    }

    fn alternatives(&self) -> impl Iterator<Item = UserId> {
        self.alternatives.iter().copied()
    }

    fn schedule_channel(&self) -> Option<GenericChannelId> {
        self.schedule_channel
    }

    fn alt_message(&self) -> Option<MessageId> {
        self.alt_message
    }
}

impl From<PostRow> for PostBuilder {
    fn from(value: PostRow) -> Self {
        Self {
            id: ThreadId::new(as_u64(value.id)),
            owner: value.owner(),
            activity: value.activity,
            start_time: value.start_time.to_jiff().to_zoned(TimeZone::UTC),
            description: value.description,
            fireteam_size: value.fireteam_size,
            fireteam: value
                .fireteam
                .into_iter()
                .map(|id| UserId::new(as_u64(id)))
                .collect(),
            alternatives: value
                .alternatives
                .into_iter()
                .map(|id| UserId::new(as_u64(id)))
                .collect(),
            schedule_channel: value
                .alt_channel
                .map(|id| GenericChannelId::new(as_u64(id))),
            alt_message: value.alt_message.map(|id| MessageId::new(as_u64(id))),
        }
    }
}

#[derive(Debug, Clone, FromRow)]

pub struct PostRow {
    pub id: i64,
    pub owner_id: i64,
    pub activity: String,
    pub start_time: Timestamp,
    pub description: String,
    pub fireteam_size: i16,
    pub fireteam: Vec<i64>,
    pub alternatives: Vec<i64>,
    pub alt_channel: Option<i64>,
    pub alt_message: Option<i64>,
}

impl PostRow {
    #[must_use]
    pub const fn thread(&self) -> ThreadId {
        ThreadId::new(as_u64(self.id))
    }

    #[must_use]
    pub const fn message(&self) -> MessageId {
        MessageId::new(as_u64(self.id))
    }

    #[must_use]
    pub const fn owner(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    pub async fn exists(pool: &PgPool, id: GenericChannelId) -> sqlx::Result<bool> {
        sqlx::query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM lfg_posts WHERE id = $1)",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
        .map(|exists| exists.unwrap_or(false))
    }

    pub async fn fetch_owner(
        pool: &PgPool,
        id: GenericChannelId,
    ) -> sqlx::Result<UserId> {
        sqlx::query_scalar!(
            "SELECT owner_id from lfg_posts WHERE id = $1",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
        .map(|id| UserId::new(as_u64(id)))
    }

    pub async fn get(pool: &PgPool, id: GenericChannelId) -> sqlx::Result<Self> {
        sqlx::query_file_as!(
            PostRow,
            "sql/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
    }

    pub async fn join(
        pool: &PgPool,
        id: GenericChannelId,
        user: UserId,
        alternative: bool,
    ) -> Result<Self> {
        let mut tx = pool.begin().await?;

        sqlx::query_file!("sql/PostManager/lock_post.sql", as_i64(id.get()))
            .fetch_optional(&mut *tx)
            .await?;

        sqlx::query_file_as!(
            PostRow,
            "sql/PostManager/join.sql",
            as_i64(id.get()),
            as_i64(user.get()),
            alternative
        )
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_file_as!(
            PostRow,
            "sql/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(&mut *tx)
        .await?;

        if !alternative && row.fireteam_len() > Join::fireteam_size(&row) {
            return Err(LfgError::FireteamFull);
        }

        tx.commit().await?;

        Ok(row)
    }

    pub async fn leave(
        pool: &PgPool,
        id: GenericChannelId,
        user: UserId,
    ) -> sqlx::Result<Self> {
        let mut tx = pool.begin().await?;

        sqlx::query_file_as!(
            PostRow,
            "sql/PostManager/leave.sql",
            as_i64(id.get()),
            as_i64(user.get()),
        )
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_file_as!(
            PostRow,
            "sql/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(row)
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
    )]
    pub async fn edit(pool: &PgPool, post: &Self) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/PostManager/edit.sql",
            post.id,
            post.owner_id,
            post.activity,
            post.start_time as Timestamp,
            post.description,
            post.fireteam_size,
        )
        .execute(pool)
        .await
    }

    pub async fn delete(
        pool: &PgPool,
        id: GenericChannelId,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!("DELETE FROM lfg_posts WHERE id = $1", as_i64(id.get()))
            .execute(pool)
            .await
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
    )]
    pub async fn save(pool: &PgPool, row: Self) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let main_result = sqlx::query_file!(
            "sql/PostManager/save_post.sql",
            row.id,
            row.owner_id,
            row.activity,
            row.start_time as Timestamp,
            row.description,
            row.fireteam_size,
            &row.fireteam,
            &row.alternatives
        )
        .execute(&mut *tx)
        .await?;

        if let (Some(channel), Some(message)) = (row.alt_channel, row.alt_message) {
            sqlx::query!(
                "INSERT INTO lfg_messages (post_id, message_id, channel_id) VALUES ($1, $2, $3) ON CONFLICT (post_id) DO NOTHING",
                row.id,
                message,
                channel,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(main_result)
    }
}

impl Join for PostRow {
    fn fireteam_size(&self) -> i16 {
        self.fireteam_size
    }

    fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(as_u64(id)))
    }

    fn fireteam_len(&self) -> i16 {
        i16::try_from(self.fireteam.len()).unwrap_or(i16::MAX)
    }

    fn alternatives(&self) -> impl Iterator<Item = UserId> {
        self.alternatives.iter().map(|&id| UserId::new(as_u64(id)))
    }
}

impl TemplateInfo for PostRow {
    fn activity(&self) -> &str {
        &self.activity
    }

    fn timestamp(&self) -> jiff::Timestamp {
        self.start_time.to_jiff()
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn fireteam_size(&self) -> i16 {
        self.fireteam_size
    }

    fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(as_u64(id)))
    }

    fn alternatives(&self) -> impl Iterator<Item = UserId> {
        self.alternatives.iter().map(|&id| UserId::new(as_u64(id)))
    }

    fn schedule_channel(&self) -> Option<GenericChannelId> {
        self.alt_channel.map(|id| GenericChannelId::new(as_u64(id)))
    }

    fn alt_message(&self) -> Option<MessageId> {
        self.alt_message.map(|id| MessageId::new(as_u64(id)))
    }
}
