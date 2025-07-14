use async_trait::async_trait;
use serenity::all::{Context, Guild, Message, PartialGuildThread, Reaction, VoiceState};
use sqlx::{Database, Pool};

#[async_trait]
pub trait GuildCreate<E: std::error::Error> {
    async fn run<Db: Database>(ctx: &Context, guild: &Guild, pool: &Pool<Db>) -> Result<(), E>;
}

#[async_trait]
pub trait MessageCreate<E: std::error::Error> {
    async fn run<Db: Database>(ctx: &Context, message: &Message, pool: &Pool<Db>) -> Result<(), E>;
}

#[async_trait]
pub trait ReactionAdd<E: std::error::Error> {
    async fn run<Db: Database>(
        ctx: &Context,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait ReactionRemove<E: std::error::Error> {
    async fn run<Db: Database>(
        ctx: &Context,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait Ready<E: std::error::Error> {
    async fn run<Db: Database>(
        ctx: &Context,
        ready: &serenity::all::Ready,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait VoiceStateUpdate<E: std::error::Error> {
    async fn run<Db: Database>(
        ctx: &Context,
        voice_state: &VoiceState,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait ThreadDelete<E: std::error::Error> {
    async fn run<Db: Database>(
        ctx: &Context,
        thread: &PartialGuildThread,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}
