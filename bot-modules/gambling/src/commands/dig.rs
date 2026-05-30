use std::collections::HashMap;
use std::sync::LazyLock;

use async_trait::async_trait;
use jiff_sqlx::{Timestamp, ToSqlx};
use rand::rng;
use rand_distr::{Binomial, Distribution};
use serenity::all::{
    Colour,
    CommandInteraction,
    Context,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum};

use super::Commands;
use crate::events::{Dispatch, Event};
use crate::models::{MineAmount, Prestige};
use crate::shop::ShopCurrency;
use crate::{
    Coins,
    Gems,
    GoalsManager,
    MaxBet,
    MineHourly,
    Result,
    Stamina,
    StaminaManager,
};

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
    async fn row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<DigRow>>;

    async fn save(pool: &Pool<Db>, row: &DigRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(Debug, FromRow)]
pub struct DigRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub level: i32,
    pub miners: i64,
    pub coal: i64,
    pub iron: i64,
    pub gold: i64,
    pub redstone: i64,
    pub lapis: i64,
    pub diamonds: i64,
    pub emeralds: i64,
    pub prestige: i64,
    pub mine_activity: Timestamp,
}

impl DigRow {
    #[must_use]
    pub fn new(id: UserId) -> Self {
        Self {
            user_id: id.get().cast_signed(),
            coins: 0,
            gems: 0,
            stamina: 0,
            level: 0,
            miners: 0,
            coal: 0,
            iron: 0,
            gold: 0,
            redstone: 0,
            lapis: 0,
            diamonds: 0,
            emeralds: 0,
            prestige: 0,
            mine_activity: jiff::Timestamp::now().to_sqlx(),
        }
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
        self.prestige
    }
}

impl MaxBet for DigRow {
    fn level(&self) -> i32 {
        self.level
    }
}

impl MineHourly for DigRow {
    fn miners(&self) -> i64 {
        self.miners
    }
}

impl MineAmount for DigRow {
    fn mine_activity(&self) -> jiff::Timestamp {
        self.mine_activity.to_jiff()
    }
}

impl Commands {
    pub async fn dig<
        Data: EmojiCacheData,
        Db: Database,
        StaminaHandler: StaminaManager<Db>,
        GoalsHandler: GoalsManager<Db> + Send + Sync,
        DigHandler: DigManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut row = DigHandler::row(pool, interaction.user.id)
            .await
            .expect("async call")
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

        let miners = (row.miners() * 10) * row.prestige_mult_10() / 10;

        for (&resource, chance) in CHANCES.iter() {
            let ore = Binomial::new(miners.cast_unsigned(), (chance).min(1.0))
                .expect("miners >= 0 and chance in [0, 1]")
                .sample(&mut rng())
                .cast_signed();

            *resources.get_mut(resource).expect("resource key in map") +=
                match resource {
                    "lapis" => ore * 6,    // Drops per ore
                    "redstone" => ore * 4, // Drops per ore
                    _ => ore,
                };
        }

        for (&k, &v) in &resources {
            match k {
                "coal" => row.coal += v,
                "iron" => row.iron += v,
                "gold" => row.gold += v,
                "redstone" => row.redstone += v,
                "lapis" => row.lapis += v,
                "diamonds" => row.diamonds += v,
                "emeralds" => row.emeralds += v,
                _ => {},
            }
        }

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool, &emojis)
            .fire(interaction.channel_id, &mut row, Event::Work(interaction.user.id))
            .await?;

        let mine_amount = row.mine_amount();
        *row.coins_mut() += mine_amount;

        row.done_work();
        row.mine_activity = jiff::Timestamp::now().to_sqlx();

        let stamina = row.stamina_str();

        DigHandler::save(pool, &row).await?;

        let found = resources
            .drain()
            .filter(|(_, v)| *v > 0)
            .filter_map(|(k, v)| match k {
                "coal" => Some((ShopCurrency::Coal, v, k)),
                "iron" => Some((ShopCurrency::Iron, v, k)),
                "gold" => Some((ShopCurrency::Gold, v, k)),
                "redstone" => Some((ShopCurrency::Redstone, v, k)),
                "lapis" => Some((ShopCurrency::Lapis, v, k)),
                "diamonds" => Some((ShopCurrency::Diamonds, v, k)),
                "emeralds" => Some((ShopCurrency::Emeralds, v, k)),
                _ => None,
            })
            .map(|(currency, amount, name)| {
                format!("{} `{}` {name}", currency.emoji(&emojis), amount.format())
            })
            .collect::<Vec<_>>();

        let coin = emojis.emoji("heads").expect("emoji 'heads' in cache");

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
                            "\n\nWhile you were gone, your mine made:\n<:coin:{coin}> `{}` coins",
                            mine_amount.format()
                        )
                    }
                }
            ))
            .color(Colour::GOLD);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_dig<'a>() -> CreateCommand<'a> {
        CreateCommand::new("dig")
            .description("Dig in the mines to collect resources")
    }
}
