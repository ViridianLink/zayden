use super::MarathonClient;
use crate::error::Result;
use crate::model::Schedule;
use crate::schedule_rules;

impl MarathonClient {
    pub fn schedule(&self) -> Result<Schedule> {
        let (ranked_window, cryo_window) = schedule_rules::current_windows();

        Ok(Schedule {
            ranked_window,
            cryo_window,
            duo_map_pool: schedule_rules::DUO_MAP_POOL
                .iter()
                .map(ToString::to_string)
                .collect(),
            weekly_game_mode: None,
        })
    }
}
