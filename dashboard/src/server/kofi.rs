use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {crate::server::auth::current_user_id, crate::util::email_hash, sqlx::PgPool};

#[server]
pub async fn link_kofi_email(email: String) -> Result<(), ServerFnError> {
    let trimmed = email.trim().to_lowercase();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err(ServerFnError::ServerError("invalid email".to_string()));
    }

    let discord_user_id = current_user_id().await?;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };

    let email_hash = email_hash(&email);

    match sqlx::query!(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)",
        &email_hash,
        discord_user_id,
    )
    .execute(&pool)
    .await
    {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("kofi_links_email_hash_key") =>
        {
            Err(ServerFnError::ServerError(
                "This Ko-fi email is already linked to an account.".to_string(),
            ))
        },
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}
