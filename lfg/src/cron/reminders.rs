use chrono::{Datelike, Duration, Timelike};
use futures::future;
use serenity::all::{Colour, Context, CreateEmbed, CreateMessage, Http, Mentionable, ThreadId};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{CronJob, CronJobData};

use crate::{Join, PostManager, PostRow};

pub async fn create_reminders<Data: CronJobData<Db>, Db: Database, Manager: PostManager<Db>>(
    ctx: &Context,
    row: &PostRow,
) {
    let post_id = row.thread();

    let week = row.start_time - Duration::days(7);
    let day = row.start_time - Duration::hours(24);
    let mins_30 = row.start_time - Duration::minutes(30);

    let week_job = CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            week.minute(),
            week.hour(),
            week.day(),
            week.month(),
            week.year()
        ),
    )
    .set_action(move |ctx, pool| async move {
        reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
    });

    let day_job = CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            day.minute(),
            day.hour(),
            day.day(),
            day.month(),
            day.year()
        ),
    )
    .set_action(move |ctx, pool| async move {
        reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
    });

    let mins_30_job = CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            mins_30.minute(),
            mins_30.hour(),
            mins_30.day(),
            mins_30.month(),
            mins_30.year()
        ),
    )
    .set_action(move |ctx, pool| async move {
        reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
    });

    let now_job = CronJob::<Db>::new(
        format!("lfg_{post_id}"),
        &format!(
            "0 {} {} {} {} * {}",
            row.start_time.minute(),
            row.start_time.hour(),
            row.start_time.day(),
            row.start_time.month(),
            row.start_time.year()
        ),
    )
    .set_action(move |ctx, pool| async move {
        reminder::<Db, Manager>(&ctx.http, pool, post_id).await;
    });

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
    let post = match Manager::row(&pool, id).await {
        Ok(post) => post,
        Err(sqlx::Error::RowNotFound) => {
            println!("Post for '{id}' not found");
            return;
        }
        Err(e) => panic!("{e:?}"),
    };

    let timestamp = post.start_time.timestamp();

    let embed = CreateEmbed::new()
        .title(format!("{} - <t:{timestamp}>", &post.activity))
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
