use sqlx::PgPool;
use topcoat::context::Cx;

use super::context::db_pool;
use super::error::AuthError;
use super::session::current_user_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebRole {
    Admin,
    Operator,
}

impl WebRole {
    /// The literal stored in `web_user_roles.role`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
        }
    }
}

pub async fn has_role(
    pool: &PgPool,
    user_id: i64,
    role: WebRole,
) -> Result<bool, AuthError> {
    Ok(sqlx::query_scalar!(
        "SELECT 1 FROM web_user_roles WHERE discord_user_id = $1 AND role = $2",
        user_id,
        role.as_str(),
    )
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())?)
}

/// The signed-in user's id if they hold `role`; otherwise
/// [`AuthError::Unauthenticated`] or [`AuthError::Forbidden`].
pub async fn require_role(cx: &Cx, role: WebRole) -> Result<i64, AuthError> {
    let user_id = current_user_id(cx).await?;
    let pool = db_pool(cx)?;

    if has_role(pool, user_id, role).await? {
        Ok(user_id)
    } else {
        Err(AuthError::Forbidden)
    }
}
