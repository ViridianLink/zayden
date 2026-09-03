use std::sync::Arc;

use serenity::all::{CreateMessage, GuildId, Http, Mentionable, ThreadId, UserId};
use tracing::{error, warn};
use zayden_app::state::AppState;

use crate::faq::{FaqContext, keywords, lookup, triage};

pub(crate) fn on_ticket_opened(
    http: Arc<Http>,
    app: Arc<AppState>,
    context: FaqContext,
    thread_id: ThreadId,
    guild_id: GuildId,
    author: UserId,
    content: String,
) {
    tokio::spawn(run_triage(
        http, app, context, thread_id, guild_id, author, content,
    ));
}

async fn run_triage(
    http: Arc<Http>,
    app: Arc<AppState>,
    context: FaqContext,
    thread_id: ThreadId,
    guild_id: GuildId,
    author: UserId,
    content: String,
) {
    let keywords = match keywords::extract(&app, &content).await {
        Ok(keywords) => keywords,
        Err(e) => {
            error!(error = ?e, %thread_id, "faq triage keyword extraction failed");
            return;
        },
    };

    let results = lookup::search_keywords(
        &app.db,
        guild_id,
        &app.http,
        &context.wiki,
        &keywords,
    )
    .await;

    let triage = match triage::synthesize(&app, &content, &results).await {
        Ok(triage) => triage,
        Err(e) => {
            error!(error = ?e, %thread_id, "faq triage synthesis failed");
            return;
        },
    };

    let embed = triage::embed(&context.wiki, &triage, &results);

    if let Err(e) = thread_id
        .widen()
        .send_message(
            &http,
            CreateMessage::new().content(author.mention().to_string()).embed(embed),
        )
        .await
    {
        warn!(error = ?e, %thread_id, "failed to post faq triage");
    }
}
