use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::server::auth::{WebRole, current_user_id, db_pool, has_role};

#[server]
pub async fn is_admin() -> Result<bool, ServerFnError> {
    let Ok(user_id) = current_user_id().await else {
        return Ok(false);
    };
    let pool = db_pool()?;

    has_role(&pool, user_id, WebRole::Admin).await
}
