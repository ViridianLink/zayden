use std::sync::Arc;

use serenity::all::{CreateMessage, GuildId, Http, Mentionable, ThreadId, UserId};
use tracing::{error, warn};
use zayden_app::state::AppState;

use crate::faq::triage::Opening;
use crate::faq::{FaqContext, keywords, linked, lookup, triage};

pub(crate) struct TicketOpening {
    pub thread_id: ThreadId,
    pub guild_id: GuildId,
    pub author: UserId,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub(crate) fn on_ticket_opened(
    http: Arc<Http>,
    app: Arc<AppState>,
    context: FaqContext,
    opening: TicketOpening,
) {
    tokio::spawn(run_triage(http, app, context, opening));
}

async fn run_triage(
    http: Arc<Http>,
    app: Arc<AppState>,
    context: FaqContext,
    opening: TicketOpening,
) {
    let TicketOpening { thread_id, guild_id, author, title, tags, content } =
        opening;

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

    let links = linked::pages(&app.http, &content).await;

    let triage = match triage::synthesize(
        &app,
        Opening { title: &title, tags: &tags, message: &content, links: &links },
        &results,
    )
    .await
    {
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
