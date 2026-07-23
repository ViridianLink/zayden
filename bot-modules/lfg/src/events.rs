use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use serenity::all::{
    Context,
    DiscordJsonError,
    EditThread,
    ErrorResponse,
    Guild,
    Http,
    HttpError,
    JsonErrorCode,
    PartialGuildThread,
};
use sqlx::{Database, Pool};
use tracing::debug;
use zayden_core::CronJobData;

use crate::cron::create_reminders;
use crate::templates::TemplateInfo;
use crate::{GuildManager, LfgError, PostManager, Result, actions};

pub async fn thread_delete<Db: Database, Manager: PostManager<Db>>(
    http: &Http,
    thread: &PartialGuildThread,
    pool: &Pool<Db>,
) -> Result<()> {
    if Manager::exists(pool, thread.id.widen()).await? {
        actions::delete::<Db, Manager>(http, thread.id, pool).await?;
    }

    Ok(())
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
) -> Result<()> {
    let Ok(Some(guild_row)) = GuildHandler::row(pool, guild.id).await else {
        debug!("guild not configured for LFG");
        return Ok(());
    };

    let Some(lfg_channel) = guild_row.channel_id() else {
        debug!("no LFG channel configured");
        return Ok(());
    };

    let archived_threads = match lfg_channel
        .get_archived_public_threads(&ctx.http, None, Some(100))
        .await
    {
        Ok(threads) => threads,
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error:
                    DiscordJsonError {
                        code:
                            JsonErrorCode::UnknownChannel | JsonErrorCode::MissingAccess,
                        ..
                    },
                ..
            },
        ))) => {
            debug!("bot lacks channel access to LFG channel");
            return Ok(());
        },
        Err(e) => return Err(LfgError::Serenity(e)),
    };

    let threads = guild
        .threads
        .iter()
        .filter(|thread| thread.parent_id == lfg_channel)
        .chain(archived_threads.threads.iter())
        .cloned();

    let now = Timestamp::now().to_zoned(TimeZone::UTC);
    let week_ago = &now - Span::new().days(7);
    let month_ago = &now - Span::new().days(30);

    for mut thread in threads {
        let Some(last_message_id) = thread.base.last_message_id else {
            continue;
        };
        let Ok(created_at) =
            Timestamp::from_second(last_message_id.created_at().unix_timestamp())
        else {
            continue;
        };
        let created_at = created_at.to_zoned(TimeZone::UTC);

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
                Err(e) => return Err(e.into()),
            }
        }

        if created_at < week_ago {
            match thread.edit(&ctx.http, EditThread::new().archived(true)).await {
                Ok(())
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                    ErrorResponse {
                        error:
                            DiscordJsonError {
                                code: JsonErrorCode::UnknownChannel, ..
                            },
                        ..
                    },
                ))) => {},
                Err(e) => return Err(e.into()),
            }
        }

        let Ok(post) = PostHandler::post_row(pool, thread.id.widen()).await else {
            continue;
        };

        let start_time = post.start_time.to_jiff().to_zoned(TimeZone::UTC);

        if start_time > now {
            create_reminders::<Data, Db, PostHandler>(ctx, &post).await;
        }

        if start_time < now
            && let (Some(channel), Some(message)) =
                (post.schedule_channel(), post.alt_message())
        {
            match channel
                .delete_message(&ctx.http, message, Some("Expired LFG post"))
                .await
            {
                Ok(())
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                    ErrorResponse {
                        error:
                            DiscordJsonError {
                                code:
                                    JsonErrorCode::UnknownMessage
                                    | JsonErrorCode::UnknownChannel,
                                ..
                            },
                        ..
                    },
                ))) => {},
                Err(e) => return Err(e.into()),
            }
        }

        if start_time + Span::new().hours(2) < now {
            match post
                .thread()
                .edit(&ctx.http, EditThread::new().archived(true))
                .await
            {
                Ok(_)
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                    ErrorResponse {
                        error:
                            DiscordJsonError {
                                code: JsonErrorCode::UnknownChannel, ..
                            },
                        ..
                    },
                ))) => {},
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}
