use std::sync::Arc;

use serenity::all::{CreateMessage, GuildId, Http, Mentionable, ThreadId, UserId};
use tracing::{error, warn};
use zayden_app::state::AppState;

use crate::Result;
use crate::faq::triage::Opening;
use crate::faq::{
    FaqContext,
    essential,
    keywords,
    linked,
    lookup,
    screenshots,
    triage,
};

pub(crate) struct TicketOpening {
    pub thread_id: ThreadId,
    pub guild_id: GuildId,
    pub author: UserId,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
    pub images: Vec<String>,
}

pub(crate) fn on_ticket_opened(
    http: Arc<Http>,
    app: Arc<AppState>,
    context: FaqContext,
    opening: TicketOpening,
) {
    tokio::spawn(async move {
        let thread_id = opening.thread_id;

        if let Err(e) = triage_ticket(&http, &app, &context, opening).await {
            error!(error = ?e, %thread_id, "faq triage failed");
        }
    });
}

pub(crate) async fn triage_ticket(
    http: &Http,
    app: &AppState,
    context: &FaqContext,
    opening: TicketOpening,
) -> Result<()> {
    let TicketOpening { thread_id, guild_id, author, title, tags, content, images } =
        opening;

    let screenshots = if images.is_empty() {
        String::new()
    } else {
        screenshots::read(app, &title, images).await.unwrap_or_else(|e| {
            warn!(error = ?e, %thread_id, "faq triage could not read screenshots");
            String::new()
        })
    };

    let query = keywords::query(&title, &content, &screenshots);
    let keywords = keywords::extract(app, &query).await?;

    let results = lookup::search_keywords(
        &app.db,
        guild_id,
        &app.http,
        &context.wiki,
        &keywords,
    )
    .await;

    let (links, essential) = tokio::join!(
        linked::pages(&app.http, &content),
        essential::pages(&app.http, &context.wiki),
    );

    let triage = triage::synthesize(
        app,
        Opening {
            title: &title,
            tags: &tags,
            message: &content,
            screenshots: &screenshots,
            links: &links,
        },
        &results,
    )
    .await?;

    let embed = triage::embed(&context.wiki, &triage, &results, &essential);

    thread_id
        .widen()
        .send_message(
            http,
            CreateMessage::new().content(author.mention().to_string()).embed(embed),
        )
        .await?;

    Ok(())
}
