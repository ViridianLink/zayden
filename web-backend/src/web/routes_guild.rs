use std::env;
use std::time::Duration;

use axum::Json;
use axum::extract::Path;
use serde::{Deserialize, Serialize};
use twilight_http::Client;
use twilight_model::guild::Guild;
use twilight_model::id::Id;

use crate::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct GuildResponse(Guild);

pub async fn guild(Path(id): Path<String>) -> Result<Json<Option<GuildResponse>>> {
    // Recieve user auth-token

    let client = Client::builder()
        .token(env::var("DISCORD_TOKEN").unwrap())
        .timeout(Duration::from_secs(20))
        .build();
    let guild_id = Id::new(id.parse().unwrap());

    match client.guild(guild_id).with_counts(true).await {
        Ok(response) => {
            let guild = response.model().await.unwrap();
            Ok(Json(Some(GuildResponse(guild))))
        }
        Err(e) => Ok(Json(None)),
    }
}
