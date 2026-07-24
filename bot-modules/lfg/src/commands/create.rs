use std::collections::HashMap;

use jiff::Timestamp;
use serenity::all::{
    CommandInteraction,
    CreateInteractionResponse,
    CreateModal,
    Http,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::required_option;

use super::Command;
use crate::modals::modal_components;
use crate::{ACTIVITIES, Result, UserSettings};

impl Command {
    pub async fn create(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let activity: &str = required_option(&mut options, "activity")?;

        let template = match options.remove("template") {
            Some(ResolvedValue::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        };

        let timezone =
            UserSettings::get(pool, interaction.user.id, &interaction.locale)
                .await?;
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
