use std::collections::HashMap;

use jiff::tz;
use jiff::tz::TimeZone;
use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use super::Command;
use crate::{Result, TimezoneManager};

impl Command {
    pub async fn timezone<Db: Database, Manager: TimezoneManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let Some(ResolvedValue::String(region)) = options.remove("region") else {
            return Ok(());
        };

        let tz = tz::db().get(region).unwrap_or(TimeZone::UTC);
        let tz_name = tz.iana_name().unwrap_or(region);

        Manager::save(pool, interaction.user.id, tz_name).await?;

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
