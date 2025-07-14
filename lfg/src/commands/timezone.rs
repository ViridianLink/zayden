use std::collections::HashMap;
use std::str::FromStr;

use chrono_tz::Tz;
use serenity::all::{CommandInteraction, EditInteractionResponse, Http, ResolvedValue};
use sqlx::{Database, Pool};

use crate::{Result, TimezoneManager};

use super::Command;

impl Command {
    pub async fn timezone<Db: Database, Manager: TimezoneManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await.unwrap();

        let Some(ResolvedValue::String(region)) = options.remove("region") else {
            unreachable!("Region is required");
        };

        let tz = Tz::from_str(region).unwrap();

        Manager::save(pool, interaction.user.id, tz).await.unwrap();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("Your timezone has been set to {}", tz.name())),
            )
            .await
            .unwrap();

        Ok(())
    }
}
