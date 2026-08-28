use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{
        GuildAccess,
        WebRole,
        current_user_id,
        db_pool,
        discord_client,
        guild_admin_context,
        has_role,
        require_role,
        server_err,
    },
    twilight_model::id::Id,
    twilight_model::id::marker::GuildMarker,
};

use crate::dto::GuildInfo;

#[cfg(feature = "ssr")]
const GUILD_PAGE_LIMIT: u16 = 200;

#[server]
pub async fn is_operator() -> Result<bool, ServerFnError> {
    let Ok(user_id) = current_user_id().await else {
        return Ok(false);
    };
    let pool = db_pool()?;

    has_role(&pool, user_id, WebRole::Operator).await
}

#[server]
pub async fn guild_operator_access(guild: String) -> Result<bool, ServerFnError> {
    let Ok(user_id) = current_user_id().await else {
        return Ok(false);
    };
    let pool = db_pool()?;

    if !has_role(&pool, user_id, WebRole::Operator).await? {
        return Ok(false);
    }

    let ctx = guild_admin_context(&guild).await?;

    Ok(ctx.access == GuildAccess::Operator)
}

#[server]
pub async fn list_bot_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    require_role(WebRole::Operator).await?;

    let http = discord_client()?;

    let mut guilds: Vec<GuildInfo> = Vec::new();
    let mut after: Option<Id<GuildMarker>> = None;

    loop {
        let mut request = http.current_user_guilds().limit(GUILD_PAGE_LIMIT);
        if let Some(id) = after {
            request = request.after(id);
        }

        let page =
            request.await.map_err(server_err)?.model().await.map_err(server_err)?;

        // A short page is the last page
        let exhausted = page.len() < usize::from(GUILD_PAGE_LIMIT);
        after = page.last().map(|g| g.id);

        guilds.extend(page.into_iter().map(|g| GuildInfo {
            id: g.id.to_string(),
            name: g.name,
            icon: g.icon.map(|hash| hash.to_string()),
        }));

        if exhausted {
            break;
        }
    }

    guilds.sort_by_key(|g| g.name.to_lowercase());

    Ok(guilds)
}
