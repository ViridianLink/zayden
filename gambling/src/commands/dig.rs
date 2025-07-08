use std::{collections::HashMap, sync::LazyLock};

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use rand::rng;
use rand_distr::{Binomial, Distribution};
use serenity::all::{
    Colour, CommandInteraction, Context, CreateCommand, CreateEmbed, EditInteractionResponse,
    UserId,
};
use sqlx::{Database, Pool, any::AnyQueryResult, prelude::FromRow};
use zayden_core::FormatNum;

use crate::events::{Dispatch, Event};
use crate::models::{MineAmount, Prestige};
use crate::shop::ShopCurrency;
use crate::{COIN, Coins, Gems, GoalsManager, MaxBet, MineHourly, Result, Stamina, StaminaManager};

use super::Commands;

const CHUNK_BLOCKS: f64 = 16.0 * 16.0 * 62.0;
const COAL_PER_CHUNK: f64 = 140.0;
const IRON_PER_CHUNK: f64 = 77.0;
const GOLD_PER_CHUNK: f64 = 25.0;
const REDSTONE_PER_CHUNK: f64 = 7.5;
const LAPIS_PER_CHUNK: f64 = 3.4;
const DIAMOND_PER_CHUNK: f64 = 3.7;
const EMERALDS_PER_CHUNK: f64 = 3.0;

static CHANCES: LazyLock<HashMap<&str, f64>> = LazyLock::new(|| {
    HashMap::from([
        ("coal", (COAL_PER_CHUNK / CHUNK_BLOCKS)),
        ("iron", (IRON_PER_CHUNK / CHUNK_BLOCKS)),
        ("gold", (GOLD_PER_CHUNK / CHUNK_BLOCKS)),
        ("redstone", (REDSTONE_PER_CHUNK / CHUNK_BLOCKS)),
        ("lapis", (LAPIS_PER_CHUNK / CHUNK_BLOCKS)),
        ("diamonds", (DIAMOND_PER_CHUNK / CHUNK_BLOCKS)),
        ("emeralds", (EMERALDS_PER_CHUNK / CHUNK_BLOCKS)),
    ])
});

#[async_trait]
pub trait DigManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<DigRow>>;

    async fn save(pool: &Pool<Db>, row: DigRow) -> sqlx::Result<AnyQueryResult>;
}

#[derive(Debug, FromRow)]
pub struct DigRow {
    pub id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub level: Option<i32>,
    pub miners: Option<i64>,
    pub coal: Option<i64>,
    pub iron: Option<i64>,
    pub gold: Option<i64>,
    pub redstone: Option<i64>,
    pub lapis: Option<i64>,
    pub diamonds: Option<i64>,
    pub emeralds: Option<i64>,
    pub prestige: Option<i64>,
    pub mine_activity: Option<NaiveDateTime>,
}

impl DigRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
            gems: 0,
            stamina: 0,
            level: Some(0),
            miners: Some(0),
            coal: Some(0),
            iron: Some(0),
            gold: Some(0),
            redstone: Some(0),
            lapis: Some(0),
            diamonds: Some(0),
            emeralds: Some(0),
            prestige: Some(0),
            mine_activity: Some(Utc::now().naive_utc()),
        }
    }

    fn coal_mut(&mut self) -> &mut i64 {
        self.coal.get_or_insert_default()
    }
    fn iron_mut(&mut self) -> &mut i64 {
        self.iron.get_or_insert_default()
    }
    fn gold_mut(&mut self) -> &mut i64 {
        self.gold.get_or_insert_default()
    }
    fn redstone_mut(&mut self) -> &mut i64 {
        self.redstone.get_or_insert_default()
    }
    fn lapis_mut(&mut self) -> &mut i64 {
        self.lapis.get_or_insert_default()
    }
    fn diamonds_mut(&mut self) -> &mut i64 {
        self.diamonds.get_or_insert_default()
    }
    fn emeralds_mut(&mut self) -> &mut i64 {
        self.emeralds.get_or_insert_default()
    }
}

impl Coins for DigRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for DigRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Stamina for DigRow {
    fn stamina(&self) -> i32 {
        self.stamina
    }

    fn stamina_mut(&mut self) -> &mut i32 {
        &mut self.stamina
    }
}

impl Prestige for DigRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for DigRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl MineHourly for DigRow {
    fn miners(&self) -> i64 {
        self.miners.unwrap_or_default()
    }
}

impl MineAmount for DigRow {
    fn mine_activity(&self) -> NaiveDateTime {
        self.mine_activity.unwrap_or_else(|| Utc::now().naive_utc())
    }
}

impl Commands {
    pub async fn dig<
        Db: Database,
        StaminaHandler: StaminaManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        DigHandler: DigManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(ctx).await?;

        let mut row = DigHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| DigRow::new(interaction.user.id));

        row.verify_work::<Db, StaminaHandler>()?;

        let mut resources = HashMap::from([
            ("coal", 0),
            ("iron", 0),
            ("gold", 0),
            ("redstone", 0),
            ("lapis", 0),
            ("diamonds", 0),
            ("emeralds", 0),
        ]);

        let miners = 1000 + (row.miners() * row.prestige_mult_10()) / 10;

        for (&resource, chance) in CHANCES.iter() {
            let ore = Binomial::new(miners as u64, (chance).min(1.0))
                .unwrap()
                .sample(&mut rng()) as i64;

            *resources.get_mut(resource).unwrap() += match resource {
                "lapis" => ore * 6,    // Drops per ore
                "redstone" => ore * 4, // Drops per ore
                _ => ore,
            };
        }

        resources.iter().for_each(|(&k, &v)| match k {
            "coal" => *row.coal_mut() += v,
            "iron" => *row.iron_mut() += v,
            "gold" => *row.gold_mut() += v,
            "redstone" => *row.redstone_mut() += v,
            "lapis" => *row.lapis_mut() += v,
            "diamonds" => *row.diamonds_mut() += v,
            "emeralds" => *row.emeralds_mut() += v,
            s => unreachable!("Invalid resource: {s}"),
        });

        Dispatch::<Db, GoalsHandler>::new(ctx, pool)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Work(interaction.user.id),
            )
            .await?;

        let mine_amount = row.mine_amount();
        *row.coins_mut() += mine_amount;

        row.done_work();
        row.mine_activity = Some(Utc::now().naive_utc());

        let stamina = row.stamina_str();

        DigHandler::save(pool, row).await.unwrap();

        let found = resources
            .drain()
            .filter(|(_, v)| *v > 0)
            .map(|(k, v)| match k {
                "coal" => (ShopCurrency::Coal, v, k),
                "iron" => (ShopCurrency::Iron, v, k),
                "gold" => (ShopCurrency::Gold, v, k),
                "redstone" => (ShopCurrency::Redstone, v, k),
                "lapis" => (ShopCurrency::Lapis, v, k),
                "diamonds" => (ShopCurrency::Diamonds, v, k),
                "emeralds" => (ShopCurrency::Emeralds, v, k),
                s => unreachable!("Invalid resource: {s}"),
            })
            .map(|(currency, amount, name)| format!("{currency} `{}` {name}", amount.format()))
            .collect::<Vec<_>>();

        let embed = CreateEmbed::new()
            .description(format!(
                "You dug around in the mines and found:\n{}{}\n\nStamina: {stamina}",
                {
                    if found.is_empty() {
                        String::from("Just a whole lot of boring stone...")
                    } else {
                        found.join("\n")
                    }
                },
                {
                    if mine_amount == 0 {
                        String::new()
                    } else {
                        format!(
                            "\n\nWhile you were gone, your mine made:\n<:coin:{COIN}> `{}` coins",
                            mine_amount.format()
                        )
                    }
                }
            ))
            .color(Colour::GOLD);

        interaction
            .edit_response(ctx, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_dig() -> CreateCommand {
        CreateCommand::new("dig").description("Dig in the mines to collect resources")
    }
}
