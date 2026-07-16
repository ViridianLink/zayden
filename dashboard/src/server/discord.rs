use leptos::prelude::*;

use crate::dto::{ChannelInfo, RoleInfo};

#[server]
pub async fn list_guild_channels(
    guild: String,
) -> Result<Vec<ChannelInfo>, ServerFnError> {
    use std::sync::Arc;

    use twilight_model::id::Id;

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;

    let Some(http) = use_context::<Arc<twilight_http::Client>>() else {
        return Err(ServerFnError::ServerError(
            "missing Discord client".to_string(),
        ));
    };

    let mut channels = http
        .guild_channels(Id::new(guild_id_i64.cast_unsigned()))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;
    channels.sort_by_key(|c| c.position.unwrap_or_default());

    Ok(channels
        .into_iter()
        .map(|c| ChannelInfo {
            id: c.id.to_string(),
            name: c.name.unwrap_or_default(),
            kind: c.kind,
        })
        .collect())
}

#[server]
pub async fn list_guild_roles(
    guild: String,
) -> Result<Vec<RoleInfo>, ServerFnError> {
    use std::sync::Arc;

    use twilight_model::id::Id;

    use crate::server::auth::{guild_admin_context, server_err};

    let (guild_id_i64, _user_id, _access_token) =
        guild_admin_context(&guild).await?;
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(http) = use_context::<Arc<twilight_http::Client>>() else {
        return Err(ServerFnError::ServerError(
            "missing Discord client".to_string(),
        ));
    };

    let mut roles = http
        .roles(Id::new(guild_id_u64))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;
    roles.sort_by_key(|r| std::cmp::Reverse(r.position));

    Ok(roles
        .into_iter()
        .filter(|r| r.id.get() != guild_id_u64)
        .map(|r| RoleInfo {
            id: r.id.to_string(),
            name: r.name,
            color: r.colors.primary_color,
        })
        .collect())
}
