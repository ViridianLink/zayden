use std::collections::HashMap;

use chrono::Utc;
use serenity::all::{
    CommandInteraction, CreateInteractionResponse, CreateModal, Http, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::modals::modal_components;
use crate::{ACTIVITIES, Result, TimezoneManager};

use super::Command;

impl Command {
    pub async fn create<Db: Database, Manager: TimezoneManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let Some(ResolvedValue::String(activity)) = options.remove("activity") else {
            unreachable!("Activity is required");
        };

        let template = match options.remove("template") {
            Some(ResolvedValue::String(s)) => s.parse().unwrap(),
            _ => 0,
        };

        let timezone = Manager::get(pool, interaction.user.id, &interaction.locale)
            .await
            .unwrap();
        let now = Utc::now().with_timezone(&timezone);

        let fireteam_size = match ACTIVITIES.iter().find(|a| a.name == activity) {
            Some(activity) => activity.fireteam_size,
            None => 3,
        };

        let row = modal_components(activity, now, fireteam_size, None);

        let modal =
            CreateModal::new(format!("lfg_create_{template}"), "Create Event").components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }
}
