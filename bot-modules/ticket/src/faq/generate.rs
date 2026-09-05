use std::sync::Arc;

use serenity::all::{CreateMessage, GuildId, Http, ThreadId};
use tracing::{debug, error, info, warn};
use zayden_app::state::AppState;
use zayden_core::as_i64;

use crate::faq::article::{FaqArticle, NewArticle};
use crate::faq::{FaqContext, embeds, scrub, transcript, writer};
use crate::support_guild_manager::TicketStores;

const DUPLICATE_RANK: f32 = 0.9;

pub(crate) async fn on_ticket_solved(
    http: &Arc<Http>,
    app: &Arc<AppState>,
    stores: TicketStores<'_>,
    thread_id: ThreadId,
    guild_id: GuildId,
) {
    match FaqContext::generation_enabled(stores.faq, guild_id).await {
        Ok(true) => {},
        Ok(false) => {
            debug!(%guild_id, %thread_id, "faq article generation is disabled");
            return;
        },
        Err(e) => {
            warn!(error = ?e, %guild_id, "could not load faq settings");
            return;
        },
    }

    tokio::spawn(run(Arc::clone(http), Arc::clone(app), thread_id, guild_id));
}

async fn run(
    http: Arc<Http>,
    app: Arc<AppState>,
    thread_id: ThreadId,
    guild_id: GuildId,
) {
    let Some(transcript) = transcript::collect(&http, thread_id).await else {
        debug!(%thread_id, "no usable transcript for faq generation");
        return;
    };

    let draft = match writer::draft(&app, &scrub::redact(&transcript)).await {
        Ok(draft) => draft,
        Err(e) => {
            error!(error = ?e, %thread_id, "faq article synthesis failed");
            return;
        },
    };

    if !draft.is_usable() {
        info!(%thread_id, "solved ticket held no reusable solution");
        return;
    }

    let guild = as_i64(guild_id.get());

    match FaqArticle::best_match_rank(&app.db, guild, &draft.title).await {
        Ok(Some(rank)) if rank >= DUPLICATE_RANK => {
            info!(%thread_id, title = draft.title, rank, "skipped duplicate article");
            return;
        },
        Ok(_) => {},
        Err(e) => {
            error!(error = ?e, %thread_id, "faq duplicate check failed");
            return;
        },
    }

    let new = NewArticle {
        title: &draft.title,
        summary: &draft.summary,
        content: &draft.markdown,
        category: draft.category.as_deref(),
        tags: &draft.tags,
    };

    let article = match FaqArticle::insert_generated(
        &app.db,
        guild,
        as_i64(thread_id.get()),
        new,
    )
    .await
    {
        Ok(Some(article)) => article,
        Ok(None) => {
            debug!(%thread_id, "thread already has a faq article");
            return;
        },
        Err(e) => {
            error!(error = ?e, %thread_id, "faq article insert failed");
            return;
        },
    };

    if let Err(e) = thread_id
        .widen()
        .send_message(&http, CreateMessage::new().embed(embeds::created(&article)))
        .await
    {
        warn!(error = ?e, %thread_id, "failed to announce the generated faq article");
    }
}
