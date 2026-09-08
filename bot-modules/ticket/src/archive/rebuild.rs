use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serenity::all::{
    ChannelId,
    ForumTagId,
    GetMessages,
    GuildId,
    GuildThread,
    Http,
};
use tracing::{debug, warn};
use zayden_app::config::ARCHIVE_NEVER;
use zayden_app::state::AppState;
use zayden_core::{as_i64, as_u64};

use crate::archive::{archive_in, notice};
use crate::{batch, state};

const RECENT: u8 = 10;

pub async fn rebuild(http: &Arc<Http>, app: &Arc<AppState>, guilds: &[GuildId]) {
    for guild_id in guilds {
        guild(http, app, *guild_id).await;
    }
}

async fn guild(http: &Arc<Http>, app: &Arc<AppState>, guild_id: GuildId) {
    let settings = match app.settings.support.try_get(as_i64(guild_id.get())).await {
        Ok(Some(settings)) => settings,
        Ok(None) => return,
        Err(e) => {
            warn!(error = ?e, %guild_id, "could not read support settings");
            return;
        },
    };

    if settings.solved_archive_secs == ARCHIVE_NEVER {
        return;
    }

    let Some(channel_id) =
        settings.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    else {
        return;
    };

    let solved_tag = settings.solved_tag_id.map(|id| ForumTagId::new(as_u64(id)));

    let threads = match guild_id.get_active_threads(http).await {
        Ok(threads) => threads.threads,
        Err(e) => {
            warn!(error = ?e, %guild_id, "could not list the guild's open threads");
            return;
        },
    };

    let solved = threads
        .into_iter()
        .filter(|thread| {
            thread.parent_id == channel_id && is_solved(thread, solved_tag)
        })
        .collect::<Vec<_>>();

    debug!(%guild_id, posts = solved.len(), "rebuilding solved post archives");

    batch::run(solved, move |thread| async move {
        rearm(http, &thread).await;
    })
    .await;
}

fn is_solved(thread: &GuildThread, tag: Option<ForumTagId>) -> bool {
    tag.is_some_and(|tag| thread.applied_tags.contains(&tag))
        || thread.base.name.starts_with(state::SOLVED)
}

async fn rearm(http: &Arc<Http>, thread: &GuildThread) {
    let Some(at) = deadline(http, thread).await else {
        debug!(
            thread_id = %thread.id,
            "solved post names no closing time; leaving it open",
        );
        return;
    };

    // A deadline that passed while the bot was down archives on the next tick
    // of the runtime rather than being missed entirely.
    let remaining = at.saturating_sub(Timestamp::now().as_second()).max(0);
    let delay = Duration::from_secs(u64::try_from(remaining).unwrap_or_default());

    debug!(thread_id = %thread.id, ?delay, "rearmed a solved post's archive");

    archive_in(Arc::clone(http), thread.id, delay);
}

async fn deadline(http: &Http, thread: &GuildThread) -> Option<i64> {
    let messages = thread
        .id
        .widen()
        .messages(http, GetMessages::new().limit(RECENT))
        .await
        .inspect_err(|e| {
            warn!(error = ?e, thread_id = %thread.id, "could not read a solved post");
        })
        .ok()?;

    // Discord pages newest-first, so a post solved twice keeps the later
    // deadline rather than the one it was first given.
    messages
        .iter()
        .filter(|message| message.author.bot())
        .find_map(|message| notice::parse(&message.content))
}
