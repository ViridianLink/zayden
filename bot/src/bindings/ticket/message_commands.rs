use std::sync::Arc;

use serenity::all::{Http, Message};
use ticket::SupportMessageCommand;
use zayden_app::state::AppState;

use crate::Result;

pub async fn support(
    http: &Arc<Http>,
    msg: &Message,
    app: &Arc<AppState>,
) -> Result<()> {
    SupportMessageCommand::run(http, msg, app).await?;

    Ok(())
}
