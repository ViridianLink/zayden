use leptos::prelude::*;

#[server]
pub async fn link_kofi_email(email: String) -> Result<(), ServerFnError> {
    use std::fmt::Write as _;

    use sqlx::PgPool;

    use crate::server::auth::current_user_id;

    let trimmed = email.trim().to_lowercase();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err(ServerFnError::ServerError("invalid email".to_string()));
    }

    let discord_user_id = current_user_id().await?;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };

    let email_hash = {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(trimmed.as_bytes());
        digest.iter().fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    };

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
