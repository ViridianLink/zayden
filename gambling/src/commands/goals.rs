use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CreateCommand, CreateEmbed, EditInteractionResponse, Http, UserId,
};
use sqlx::{Database, FromRow, Pool};

use crate::{COIN, Coins, GamblingGoalsRow, Gems, GoalHandler, MaxBet, Prestige, Result, tomorrow};

use super::Commands;

#[async_trait]
pub trait GoalsManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<GoalsRow>>;

    async fn full_rows(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>>;

    async fn update(
        pool: &Pool<Db>,
        rows: &[GamblingGoalsRow],
    ) -> sqlx::Result<Vec<GamblingGoalsRow>>;
}

#[derive(FromRow, Default)]
pub struct GoalsRow {
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl Coins for GoalsRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for GoalsRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Prestige for GoalsRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for GoalsRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl Commands {
    pub async fn goals<Db: Database, Manager: GoalsManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await.unwrap();

        let row = Manager::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_default();

        let mut desc =
            GoalHandler::get_user_progress::<Db, Manager>(pool, interaction.user.id, &row)
                .await
                .unwrap()
                .into_iter()
                .map(|goal| format!("{}\n\n", goal.description()))
                .collect::<String>();

        desc.push_str(&format!(
            "Reward for completing __**each goals**__: 5,000 <:coin:{COIN}>\nReward for completing __**all goals**__: 1 💎\n\nGoals reset <t:{}:R>",
            tomorrow(None)
        ));

        let embed = CreateEmbed::new().title("Daily Goals 📋").description(desc);

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_goals<'a>() -> CreateCommand<'a> {
        CreateCommand::new("goals").description("Show your daily goal progress")
    }
}
