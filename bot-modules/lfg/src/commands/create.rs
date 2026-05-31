use std::collections::HashMap;

use jiff::Timestamp;
use serenity::all::{
    CommandInteraction,
    CreateInteractionResponse,
    CreateModal,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use super::Command;
use crate::modals::modal_components;
use crate::{ACTIVITIES, Result, TimezoneManager};

impl Command {
    pub async fn create<Db: Database, Manager: TimezoneManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let Some(ResolvedValue::String(activity)) = options.remove("activity")
        else {
            return Ok(());
        };

        let template = match options.remove("template") {
            Some(ResolvedValue::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        };

        let timezone =
            Manager::get(pool, interaction.user.id, &interaction.locale).await?;
        let now = Timestamp::now().to_zoned(timezone);

        let fireteam_size = ACTIVITIES
            .iter()
            .find(|a| a.name == activity)
            .map_or(3, |a| a.fireteam_size);

        let row = modal_components(activity, &now, fireteam_size, None);

        let modal =
            CreateModal::new(format!("lfg_create_{template}"), "Create Event")
                .components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }
}
