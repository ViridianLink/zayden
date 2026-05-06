use serenity::all::{Http, Message};
use sqlx::{PgPool, Postgres};
use ticket::SupportMessageCommand;

use crate::Result;
use crate::sqlx_lib::GuildTable;

pub async fn support(http: &Http, msg: &Message, pool: &PgPool) -> Result<()> {
    SupportMessageCommand::run::<Postgres, GuildTable>(http, msg, pool).await?;

    Ok(())
}
