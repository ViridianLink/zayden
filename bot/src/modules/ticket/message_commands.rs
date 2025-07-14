use serenity::all::{Context, Message};
use sqlx::{PgPool, Postgres};
use ticket::SupportMessageCommand;

use crate::Result;
use crate::sqlx_lib::GuildTable;

pub async fn support(ctx: &Context, msg: &Message, pool: &PgPool) -> Result<()> {
    SupportMessageCommand::run::<Postgres, GuildTable>(&ctx.http, msg, pool).await?;

    Ok(())
}
