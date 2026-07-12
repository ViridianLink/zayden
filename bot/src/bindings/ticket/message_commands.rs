use serenity::all::{Http, Message};
use sqlx::PgPool;
use ticket::SupportMessageCommand;

use crate::Result;

pub async fn support(http: &Http, msg: &Message, pool: &PgPool) -> Result<()> {
    SupportMessageCommand::run(http, msg, pool).await?;

    Ok(())
}
