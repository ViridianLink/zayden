use serenity::all::{
    DiscordJsonError,
    ErrorResponse,
    GenericChannelId,
    Http,
    HttpError,
    JsonErrorCode,
};
use sqlx::PgPool;

use crate::templates::TemplateInfo;
use crate::{LfgError, PostRow, Result};

pub async fn delete(
    http: &Http,
    channel: impl Into<GenericChannelId>,
    pool: &PgPool,
) -> Result<()> {
    let channel = channel.into();

    let Ok(post) = PostRow::get(pool, channel).await else {
        return Err(LfgError::ThreadNotFound);
    };

    match post.thread().widen().delete(http, Some("Lfg post deleted")).await {
        Ok(_)
        | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error: DiscordJsonError { code: JsonErrorCode::UnknownChannel, .. },
                ..
            },
        ))) => {},
        Err(e) => return Err(e.into()),
    }

    if let (Some(channel), Some(message)) =
        (post.schedule_channel(), post.alt_message())
    {
        match channel.delete_message(http, message, Some("Lfg post deleted")).await {
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

    PostRow::delete(pool, channel).await?;

    Ok(())
}
