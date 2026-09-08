use serenity::all::{
    ChannelId,
    EditThread,
    ForumTagId,
    GuildId,
    Http,
    HttpError,
    JsonErrorCode,
    ThreadId,
};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::idle::activity::ThreadActivity;
use crate::idle::close::DueClose;
use crate::idle::notice::Notice;
use crate::idle::reminder;
use crate::idle::stale::StaleTarget;
use crate::idle::sweep::DueNudge;
use crate::state;

pub(crate) type TagRequest = (i64, GuildId, Option<ChannelId>, Option<ForumTagId>);

pub(crate) async fn resolve_tags(
    http: &Http,
    rows: &[TagRequest],
) -> Vec<(i64, Option<ForumTagId>)> {
    let mut tags: Vec<(i64, Option<ForumTagId>)> = Vec::new();

    for (guild_id, guild, channel_id, tag) in rows {
        if tags.iter().any(|(seen, _)| seen == guild_id) {
            continue;
        }

        let resolved = match *channel_id {
            Some(channel_id) => state::usable_tag(http, *guild, channel_id, *tag)
                .await
                .unwrap_or_default(),
            None => None,
        };

        tags.push((*guild_id, resolved));
    }

    tags
}

#[must_use]
pub(crate) fn tag_for(
    tags: &[(i64, Option<ForumTagId>)],
    guild_id: i64,
) -> Option<ForumTagId> {
    tags.iter().find(|(seen, _)| *seen == guild_id).and_then(|(_, tag)| *tag)
}

pub(crate) async fn nudge(http: &Http, pool: &PgPool, row: &DueNudge) {
    let Some(reminder) =
        reminder(row.ball(), row.op(), row.helper(), &row.support_roles())
    else {
        debug!(
            thread_id = row.thread_id,
            "nobody to remind; the guild has no support roles",
        );
        return;
    };

    let sent =
        row.thread().widen().send_message(http, reminder.message(row.since())).await;

    let Err(e) = sent else {
        return;
    };

    // The thread can be deleted, or the bot locked out of it, between the claim
    // and the send.
    triage(pool, row.thread(), row.thread_id, &e, "idle reminder not sent").await;
}

pub(crate) async fn close(
    http: &Http,
    pool: &PgPool,
    row: &DueClose,
    tag: Option<ForumTagId>,
) {
    let notice = Notice::new(row.op(), row.since());

    if let Err(e) = row.thread().widen().send_message(http, notice.message()).await {
        triage(pool, row.thread(), row.thread_id, &e, "close notice not sent").await;
        return;
    }

    let edit =
        match state::marking(http, row.guild(), row.thread(), tag, state::CLOSED)
            .await
        {
            Ok(edit) => edit.unwrap_or_default(),
            Err(e) => {
                warn!(
                    error = ?e,
                    thread_id = row.thread_id,
                    "could not read thread to tag it closed; archiving anyway",
                );
                EditThread::new()
            },
        };

    if let Err(e) = row.thread().edit(http, edit).await {
        triage(pool, row.thread(), row.thread_id, &e, "thread not archived").await;
    }
}

pub(crate) async fn stale(
    http: &Http,
    pool: &PgPool,
    row: &StaleTarget,
    tag: Option<ForumTagId>,
) {
    retag(http, pool, row, tag, true).await;
}

pub(crate) async fn unstale(
    http: &Http,
    pool: &PgPool,
    row: &StaleTarget,
    tag: Option<ForumTagId>,
) {
    retag(http, pool, row, tag, false).await;
}

async fn retag(
    http: &Http,
    pool: &PgPool,
    row: &StaleTarget,
    tag: Option<ForumTagId>,
    applied: bool,
) {
    let Some(tag) = tag else {
        return;
    };

    let context =
        if applied { "stale tag not applied" } else { "stale tag not removed" };

    let edit = match state::retagging(http, row.guild(), row.thread(), tag, applied)
        .await
    {
        Ok(Some(edit)) => edit,
        Ok(None) => return,
        Err(e) => {
            warn!(error = ?e, thread_id = row.thread_id, "{context}");
            return;
        },
    };

    if let Err(e) = row.thread().edit(http, edit).await {
        triage(pool, row.thread(), row.thread_id, &e, context).await;
    }
}

async fn triage(
    pool: &PgPool,
    thread: ThreadId,
    thread_id: i64,
    e: &serenity::Error,
    context: &str,
) {
    match code(e) {
        Some(&JsonErrorCode::UnknownChannel) => {
            if let Err(e) = ThreadActivity::delete(pool, thread).await {
                warn!(error = ?e, thread_id, "could not drop activity row");
            }
        },
        Some(&JsonErrorCode::MissingAccess | &JsonErrorCode::ThreadLocked) => {
            debug!(thread_id, context, "no access to the thread; pausing");

            if let Err(e) = ThreadActivity::pause(pool, thread).await {
                warn!(error = ?e, thread_id, "could not pause activity row");
            }
        },
        _ => warn!(error = ?e, thread_id, "{context}"),
    }
}

fn code(e: &serenity::Error) -> Option<&JsonErrorCode> {
    let serenity::Error::Http(HttpError::UnsuccessfulRequest(resp)) = e else {
        return None;
    };

    Some(&resp.error.code)
}
