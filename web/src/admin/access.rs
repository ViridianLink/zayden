use topcoat::context::Cx;

use crate::auth::{
    AuthError,
    GuildAccess,
    WebRole,
    current_user_id,
    db_pool,
    guild_admin_context,
    has_role,
};

async fn signed_in_role(cx: &Cx, role: WebRole) -> Result<bool, AuthError> {
    let Ok(user_id) = current_user_id(cx).await else {
        return Ok(false);
    };
    let pool = db_pool(cx)?;

    has_role(pool, user_id, role).await
}

pub async fn is_admin(cx: &Cx) -> Result<bool, AuthError> {
    signed_in_role(cx, WebRole::Admin).await
}

pub async fn is_operator(cx: &Cx) -> Result<bool, AuthError> {
    signed_in_role(cx, WebRole::Operator).await
}

pub async fn guild_operator_access(cx: &Cx, guild: &str) -> Result<bool, AuthError> {
    if !is_operator(cx).await? {
        return Ok(false);
    }

    let ctx = guild_admin_context(cx, guild).await?;

    Ok(ctx.access == GuildAccess::Operator)
}
