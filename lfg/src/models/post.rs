use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serenity::all::{GenericChannelId, MessageId, ThreadId, UserId};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};

use crate::templates::TemplateInfo;
use crate::{Join, Result};

pub struct PostBuilder {
    id: ThreadId,
    owner: UserId,
    activity: String,
    start_time: DateTime<Tz>,
    description: String,
    fireteam_size: i16,
    fireteam: Vec<UserId>,
    alternatives: Vec<UserId>,
    schedule_channel: Option<GenericChannelId>,
    alt_message: Option<MessageId>,
}

impl PostBuilder {
    pub fn new(
        owner: impl Into<UserId>,
        activity: impl Into<String>,
        start: DateTime<Tz>,
        desc: impl Into<String>,
        fireteam_size: i16,
    ) -> Self {
        let owner = owner.into();

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

    pub fn id(mut self, id: impl Into<ThreadId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn activity(mut self, activity: impl Into<String>) -> Self {
        self.activity = activity.into();
        self
    }

    pub fn fireteam_size(mut self, size: i16) -> Self {
        self.fireteam_size = size;
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn start(mut self, start: DateTime<Tz>) -> Self {
        self.start_time = start;
        self
    }

    pub fn schedule_channel(mut self, channel: GenericChannelId) -> Self {
        self.schedule_channel = Some(channel);
        self
    }

    pub fn alt_message(mut self, message: MessageId) -> Self {
        self.alt_message = Some(message);
        self
    }

    pub fn build(self) -> PostRow {
        PostRow {
            id: self.id.get() as i64,
            owner: self.owner.get() as i64,
            activity: self.activity,
            start_time: self.start_time.with_timezone(&Utc),
            description: self.description,
            fireteam_size: self.fireteam_size,
            fireteam: self
                .fireteam
                .into_iter()
                .map(|user| user.get() as i64)
                .collect(),
            alternatives: self
                .alternatives
                .into_iter()
                .map(|user| user.get() as i64)
                .collect(),
            alt_channel: self.schedule_channel.map(|channel| channel.get() as i64),
            alt_message: self.alt_message.map(|message| message.get() as i64),
        }
    }
}

impl TemplateInfo for PostBuilder {
    fn activity(&self) -> &str {
        &self.activity
    }

    fn timestamp(&self) -> i64 {
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
            id: ThreadId::new(value.id as u64),
            owner: UserId::new(value.owner as u64),
            activity: value.activity,
            start_time: value.start_time.with_timezone(&Tz::UTC),
            description: value.description,
            fireteam_size: value.fireteam_size,
            fireteam: value
                .fireteam
                .into_iter()
                .map(|id| UserId::new(id as u64))
                .collect(),
            alternatives: value
                .alternatives
                .into_iter()
                .map(|id| UserId::new(id as u64))
                .collect(),
            schedule_channel: value.alt_channel.map(|id| GenericChannelId::new(id as u64)),
            alt_message: value.alt_message.map(|id| MessageId::new(id as u64)),
        }
    }
}

#[async_trait]
pub trait PostManager<Db: Database> {
    async fn exists(pool: &Pool<Db>, id: impl Into<GenericChannelId> + Send) -> sqlx::Result<bool>;

    async fn owner(pool: &Pool<Db>, id: impl Into<GenericChannelId> + Send)
    -> sqlx::Result<UserId>;

    async fn row(pool: &Pool<Db>, id: impl Into<GenericChannelId> + Send) -> sqlx::Result<PostRow>;

    async fn join(
        pool: &Pool<Db>,
        id: impl Into<GenericChannelId> + Send,
        user: impl Into<UserId> + Send,
        alternative: bool,
    ) -> Result<PostRow>;

    async fn leave(
        pool: &Pool<Db>,
        id: impl Into<GenericChannelId> + Send,
        user: impl Into<UserId> + Send,
    ) -> sqlx::Result<PostRow>;

    async fn delete(
        pool: &Pool<Db>,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<Db::QueryResult>;
}

#[derive(Debug, FromRow)]

pub struct PostRow {
    pub id: i64,
    pub owner: i64,
    pub activity: String,
    pub start_time: DateTime<Utc>,
    pub description: String,
    pub fireteam_size: i16,
    pub fireteam: Vec<i64>,
    pub alternatives: Vec<i64>,
    pub alt_channel: Option<i64>,
    pub alt_message: Option<i64>,
}

impl PostRow {
    pub fn thread(&self) -> ThreadId {
        ThreadId::new(self.id as u64)
    }

    pub fn message(&self) -> MessageId {
        MessageId::new(self.id as u64)
    }

    pub fn owner(&self) -> UserId {
        UserId::new(self.owner as u64)
    }
}

impl Join for PostRow {
    fn fireteam_size(&self) -> i16 {
        self.fireteam_size
    }

    fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(id as u64))
    }

    fn fireteam_len(&self) -> i16 {
        self.fireteam.len() as i16
    }

    fn alternatives(&self) -> impl Iterator<Item = UserId> {
        self.alternatives.iter().map(|&id| UserId::new(id as u64))
    }
}

impl TemplateInfo for PostRow {
    fn activity(&self) -> &str {
        &self.activity
    }

    fn timestamp(&self) -> i64 {
        self.start_time.timestamp()
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn fireteam_size(&self) -> i16 {
        self.fireteam_size
    }

    fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(id as u64))
    }

    fn alternatives(&self) -> impl Iterator<Item = UserId> {
        self.alternatives.iter().map(|&id| UserId::new(id as u64))
    }

    fn schedule_channel(&self) -> Option<GenericChannelId> {
        self.alt_channel.map(|id| GenericChannelId::new(id as u64))
    }

    fn alt_message(&self) -> Option<MessageId> {
        self.alt_message.map(|id| MessageId::new(id as u64))
    }
}
