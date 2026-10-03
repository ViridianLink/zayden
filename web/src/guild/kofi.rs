use topcoat::context::Cx;

use super::error::GuildError;
use super::form::form_args;
use crate::auth::{current_user_id, db_pool};
use crate::util::email_hash;

form_args! {
    KofiEmailForm { email }
}

const EMAIL_HASH_KEY: &str = "kofi_links_email_hash_key";

pub async fn link_kofi_email(
    cx: &Cx,
    form: &KofiEmailForm,
) -> Result<(), GuildError> {
    let trimmed = form.email.trim().to_lowercase();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err(GuildError::InvalidEmail);
    }

    let discord_user_id = current_user_id(cx).await?;
    let pool = db_pool(cx)?;

    let email_hash = email_hash(&form.email);

    match sqlx::query!(
        "INSERT INTO kofi_links (email_hash, discord_user_id) VALUES ($1, $2)",
        &email_hash,
        discord_user_id,
    )
    .execute(pool)
    .await
    {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(e)) if e.constraint() == Some(EMAIL_HASH_KEY) => {
            Err(GuildError::KofiEmailTaken)
        },
        Err(e) => Err(e.into()),
    }
}
