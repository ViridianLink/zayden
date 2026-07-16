use serenity::all::{Http, Message};
use sqlx::PgPool;
use ticket::{SupportMessageCommand, TicketStores};

use crate::Result;

pub async fn support(
    http: &Http,
    msg: &Message,
    stores: TicketStores<'_>,
    pool: &PgPool,
) -> Result<()> {
    SupportMessageCommand::run(http, msg, stores, pool).await?;

    Ok(())
}
