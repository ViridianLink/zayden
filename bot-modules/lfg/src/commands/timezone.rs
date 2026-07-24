use std::collections::HashMap;

use jiff::tz;
use jiff::tz::TimeZone;
use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::required_option;

use super::Command;
use crate::{Result, UserSettings};

impl Command {
    pub async fn timezone(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let region: &str = required_option(&mut options, "region")?;

        let tz = tz::db().get(region).unwrap_or(TimeZone::UTC);
        let tz_name = tz.iana_name().unwrap_or(region);

        UserSettings::save(pool, interaction.user.id, tz_name).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("Your timezone has been set to {tz_name}")),
            )
            .await?;

        Ok(())
    }
}
