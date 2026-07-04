use serenity::all::{Http, Message};
use sqlx::{PgPool, Postgres};
use ticket::{GuildTable, SupportMessageCommand};

use crate::Result;

pub async fn support(http: &Http, msg: &Message, pool: &PgPool) -> Result<()> {
    SupportMessageCommand::run::<Postgres, GuildTable>(http, msg, pool).await?;

    Ok(())
}
