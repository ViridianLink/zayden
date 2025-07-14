use serenity::all::{
    DiscordJsonError, ErrorResponse, GenericChannelId, Http, HttpError, JsonErrorCode,
};
use sqlx::{Database, Pool};

use crate::{PostManager, Result, templates::TemplateInfo};

pub async fn delete<Db: Database, Manager: PostManager<Db>>(
    http: &Http,
    channel: impl Into<GenericChannelId>,
    pool: &Pool<Db>,
) -> Result<()> {
    let channel = channel.into();

    let Ok(post) = Manager::row(pool, channel).await else {
        return Ok(());
    };

    match post
        .thread()
        .widen()
        .delete(http, Some("Lfg post deleted"))
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

    if let (Some(channel), Some(message)) = (post.schedule_channel(), post.alt_message()) {
        match channel
            .delete_message(http, message, Some("Lfg post deleted"))
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
        }
    }

    Manager::delete(pool, channel).await.unwrap();

    Ok(())
}
