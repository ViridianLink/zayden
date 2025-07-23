use chrono::{Duration, Utc};
use serenity::all::{
    Context, DiscordJsonError, EditThread, ErrorResponse, Guild, Http, HttpError, JsonErrorCode,
    PartialGuildThread,
};
use sqlx::{Database, Pool};
use zayden_core::CronJobData;

use crate::{GuildManager, PostManager, actions, cron::create_reminders, templates::TemplateInfo};

pub async fn thread_delete<Db: Database, Manager: PostManager<Db>>(
    http: &Http,
    thread: &PartialGuildThread,
    pool: &Pool<Db>,
) {
    if Manager::exists(pool, thread.id).await.unwrap() {
        actions::delete::<Db, Manager>(http, thread.id, pool)
            .await
            .unwrap();
    }
}

pub async fn guild_create<
    Data: CronJobData<Db>,
    Db: Database,
    GuildHandler: GuildManager<Db>,
    PostHandler: PostManager<Db>,
>(
    ctx: &Context,
    guild: &Guild,
    pool: &Pool<Db>,
) {
    let Ok(Some(guild_row)) = GuildHandler::row(pool, guild.id).await else {
        return;
    };

    let lfg_channel = guild_row.channel_id();

    let archived_threads = lfg_channel
        .get_archived_public_threads(&ctx.http, None, Some(100))
        .await
        .unwrap();

    let threads = guild
        .threads
        .iter()
        .filter(|thread| thread.parent_id == lfg_channel)
        .chain(archived_threads.threads.iter())
        .cloned();

    let now = Utc::now();
    let week_ago = now - Duration::days(7);
    let month_ago = now - Duration::days(30);

    for mut thread in threads {
        let created_at = *thread.base.last_message_id.unwrap().created_at();

        if created_at < month_ago {
            match thread
                .delete(&ctx.http, Some("Thread older than 30 days"))
                .await
            {
                Ok(_)
                // Channel already removed
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::UnknownChannel,
                            ..
                        },
                    ..
                }))) => {}
                Err(e) => panic!("{e:?}"),
            };
        }

        if created_at < week_ago {
            match thread
                .edit(&ctx.http, EditThread::new().archived(true))
                .await
            {
                Ok(_)
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::UnknownChannel,
                            ..
                        },
                    ..
                }))) => {}
                Err(e) => panic!("{e:?}"),
            }
        }

        let post = match PostHandler::row(pool, thread.id).await {
            Ok(post) => post,
            Err(_) => continue,
        };

        if post.start_time > now {
            create_reminders::<Data, Db, PostHandler>(ctx, &post).await;
        }

        if post.start_time < now
            && let (Some(channel), Some(message)) = (post.schedule_channel(), post.alt_message())
        {
            match channel
                .delete_message(&ctx.http, message, Some("Expired LFG post"))
                .await
            {
                Ok(_)
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::UnknownMessage,
                            ..
                        },
                    ..
                }))) => {}
                Err(e) => panic!("{e:?}"),
            };
        }

        if post.start_time + Duration::hours(2) < now {
            post.thread()
                .edit(&ctx.http, EditThread::new().archived(true))
                .await
                .unwrap();
        }
    }
}
