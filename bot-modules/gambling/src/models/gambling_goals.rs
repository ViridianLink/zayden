use jiff::tz::TimeZone;
use jiff_sqlx::{Date, ToSqlx};
use serenity::all::UserId;
use sqlx::FromRow;
use zayden_core::{FormatNum, as_i64};

use crate::goals::GOAL_REGISTRY;

#[derive(FromRow)]
pub struct GamblingGoalsRow {
    pub user_id: i64,
    pub goal_id: String,
    pub day: Date,
    pub progress: i64,
    pub target: i64,
}

impl GamblingGoalsRow {
    pub fn new(user_id: UserId, goal_id: impl Into<String>, target: i64) -> Self {
        Self {
            user_id: as_i64(user_id.get()),
            goal_id: goal_id.into(),
            day: jiff::civil::Date::default().to_sqlx(),
            progress: 0,
            target,
        }
    }

    #[must_use]
    pub fn goal_id(&self) -> &str {
        &self.goal_id
    }

    #[must_use]
    pub fn is_today(&self) -> bool {
        self.day.to_jiff() == jiff::Timestamp::now().to_zoned(TimeZone::UTC).date()
    }

    pub fn update_progress(&mut self, value: i64) {
        self.progress += value;
        self.progress = self.progress.min(self.target);
    }

    pub const fn reset_progress(&mut self) {
        self.progress = 0;
    }

    pub const fn set_completed(&mut self) {
        self.progress = self.target;
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.progress == self.target
    }

    pub fn title(&self) -> String {
        GOAL_REGISTRY.get_definition(&self.goal_id).map_or_else(
            || self.goal_id.clone(),
            |goal| (goal.description)(self.target),
        )
    }

    #[must_use]
    pub fn description(&self) -> String {
        let title = self.title();

        let progress_str = self.progress.format();
        let target_str = self.target.format();

        if self.is_complete() {
            format!("~~**{title}**~~\nProgress: Done 🟢")
        } else {
            format!("**{title}**\nProgress: `{progress_str}/{target_str}`")
        }
    }
}
