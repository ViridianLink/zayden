use futures::future;
use jiff::Span;
use jiff::tz::TimeZone;
use serenity::all::{
    Colour,
    Context,
    CreateEmbed,
    CreateMessage,
    Http,
    Mentionable,
    ThreadId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use tracing::error;
use zayden_core::{CronJob, CronJobData};

use crate::{Join, PostManager, PostRow};

#[expect(
    clippy::significant_drop_tightening,
    reason = "jobs borrows from the write guard and must be held for the full extend call"
)]
pub async fn create_reminders<
    Data: CronJobData<Db>,
    Db: Database,
    Manager: PostManager<Db>,
>(
    ctx: &Context,
    row: &PostRow,
) {
    let post_id = row.thread();
    let start_time = row.start_time.to_jiff().to_zoned(TimeZone::UTC);

    let week = &start_time - Span::new().days(7);
    let day = &start_time - Span::new().hours(24);
    let mins_30 = &start_time - Span::new().minutes(30);

    let week_job = match CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            week.minute(),
            week.hour(),
            week.day(),
            week.month(),
            week.year()
        ),
    ) {
        Ok(j) => j.set_action(move |ctx, pool| async move {
            reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
        }),
        Err(e) => {
            error!(error = ?e, "invalid lfg cron schedule");
            return;
        },
    };

    let day_job = match CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            day.minute(),
            day.hour(),
            day.day(),
            day.month(),
            day.year()
        ),
    ) {
        Ok(j) => j.set_action(move |ctx, pool| async move {
            reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
        }),
        Err(e) => {
            error!(error = ?e, "invalid lfg cron schedule");
            return;
        },
    };

    let mins_30_job = match CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            mins_30.minute(),
            mins_30.hour(),
            mins_30.day(),
            mins_30.month(),
            mins_30.year()
        ),
    ) {
        Ok(j) => j.set_action(move |ctx, pool| async move {
            reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
        }),
        Err(e) => {
            error!(error = ?e, "invalid lfg cron schedule");
            return;
        },
    };

    let now_job = match CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            start_time.minute(),
            start_time.hour(),
            start_time.day(),
            start_time.month(),
            start_time.year()
        ),
    ) {
        Ok(j) => j.set_action(move |ctx, pool| async move {
            reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
        }),
        Err(e) => {
            error!(error = ?e, "invalid lfg cron schedule");
            return;
        },
    };

    let data = ctx.data::<RwLock<Data>>();
    let mut data = data.write().await;
    let jobs = data.jobs_mut();

    jobs.retain(|job| job.id != format!("lfg_{post_id}"));
    jobs.extend([week_job, day_job, mins_30_job, now_job]);
}

async fn reminder<Db: Database, Manager: PostManager<Db>>(
    http: &Http,
    pool: Pool<Db>,
    id: ThreadId,
) {
    let post = match Manager::post_row(&pool, id).await {
        Ok(post) => post,
        // Post deleted
        Err(sqlx::Error::RowNotFound) => return,
        Err(e) => {
            error!(error = ?e, post_id = %id, "lfg reminder: post_row lookup failed");
            return;
        },
    };

    let timestamp = post.start_time.to_jiff();

    let embed = CreateEmbed::new()
        .title(format!("{} - <t:{timestamp}>", post.activity))
        .colour(Colour::BLUE)
        .description(format!(
            "Starting <t:{timestamp}:R>\nThread: {}",
            post.thread().widen().mention()
        ))
        .field(
            "Joined",
            post.fireteam()
                .map(|user| user.mention().to_string())
                .collect::<Vec<_>>()
                .join(" | "),
            false,
        );

    let iter = post
        .fireteam()
        .map(|user| user.dm(http, CreateMessage::new().embed(embed.clone())));

    future::join_all(iter).await;
}
