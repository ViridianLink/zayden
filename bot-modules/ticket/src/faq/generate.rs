use std::sync::Arc;

use jiff::Timestamp;
use serenity::all::{GuildId, Http, ThreadId};
use tracing::{debug, error, info, warn};
use zayden_app::state::AppState;
use zayden_core::as_i64;

use crate::faq::article::{FaqArticle, NewArticle};
use crate::faq::reconcile::{Outcome, Reconciler, Subject, candidates};
use crate::faq::{FaqContext, scrub, transcript, writer};
use crate::support_guild_manager::TicketStores;

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
    let new = draft.article.as_new();

    match reconcile(&app, guild, thread_id, new).await {
        Outcome::Create => publish(&app, guild, thread_id, new).await,
        Outcome::Unresolved { reason } => {
            info!(%thread_id, reason, "faq reconcile unresolved, publishing new article");
            publish(&app, guild, thread_id, new).await;
        },
        Outcome::Discard { target, reason } => {
            info!(%thread_id, target, reason, "discarded article already covered");
        },
        Outcome::Merge { target, article, reason } => {
            match FaqArticle::merge(&app.db, guild, target, article.as_new()).await {
                Ok(Some(_)) => {
                    info!(%thread_id, target, reason, "merged article into existing");
                },
                Ok(None) => {
                    debug!(%thread_id, target, "merge target vanished");
                    publish(&app, guild, thread_id, new).await;
                },
                Err(e) => {
                    error!(error = ?e, %thread_id, target, "faq article merge failed");
                },
            }
        },
    }
}

async fn reconcile(
    app: &AppState,
    guild: i64,
    thread_id: ThreadId,
    new: NewArticle<'_>,
) -> Outcome {
    let existing = match candidates(&app.db, guild, new, None).await {
        Ok(existing) => existing,
        Err(e) => {
            warn!(error = ?e, %thread_id, "faq similar article lookup failed");
            return Outcome::Create;
        },
    };

    let subject = Subject { article: new, dated: Timestamp::now() };

    let result = match Reconciler::from_app(app) {
        Ok(reconciler) => reconciler.reconcile(subject, &existing).await,
        Err(e) => Err(e),
    };

    result.unwrap_or_else(|e| {
        warn!(error = ?e, %thread_id, "faq reconcile failed");
        Outcome::Create
    })
}

async fn publish(
    app: &AppState,
    guild: i64,
    thread_id: ThreadId,
    new: NewArticle<'_>,
) {
    match FaqArticle::insert_generated(&app.db, guild, as_i64(thread_id.get()), new)
        .await
    {
        Ok(Some(_)) => {},
        Ok(None) => {
            debug!(%thread_id, "thread already has a faq article");
        },
        Err(e) => {
            error!(error = ?e, %thread_id, "faq article insert failed");
        },
    }
}
