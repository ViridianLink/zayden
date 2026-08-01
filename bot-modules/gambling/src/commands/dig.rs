use std::collections::HashMap;
use std::sync::LazyLock;

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
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64, as_u64};

use super::Commands;
use crate::events::{Dispatch, Event};
use crate::models::{MineAmount, MinePayout, Prestige};
use crate::shop::ShopCurrency;
use crate::{
    Coins,
    GamblingError,
    Gems,
    MaxBet,
    MineHourly,
    Result,
    Stamina,
    out_of_stamina,
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

pub struct DigManager;

impl DigManager {
    pub async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<DigRow>> {
        sqlx::query_as!(
            DigRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS "level!",

                COALESCE(m.miners, 0) AS "miners!",
                COALESCE(m.coal, 0) AS "coal!",
                COALESCE(m.iron, 0) AS "iron!",
                COALESCE(m.gold, 0) AS "gold!",
                COALESCE(m.redstone, 0) AS "redstone!",
                COALESCE(m.lapis, 0) AS "lapis!",
                COALESCE(m.diamonds, 0) AS "diamonds!",
                COALESCE(m.emeralds, 0) AS "emeralds!",
                COALESCE(m.prestige, 0) AS "prestige!",
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS "mine_activity!: jiff_sqlx::Timestamp"
                
            FROM gambling g
            LEFT JOIN levels l ON g.user_id = l.user_id
            LEFT JOIN gambling_mine m ON g.user_id = m.user_id
            WHERE g.user_id = $1;"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn commit_dig(
        pool: &PgPool,
        id: UserId,
        delta: &DigDelta,
        payout: MinePayout,
    ) -> sqlx::Result<Option<DigCommit>> {
        let user_id = as_i64(id.get());

        let mut tx = pool.begin().await?;

        let Some(balance) = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
                coins = gambling.coins + $2,
                gems = gambling.gems + $3,
                stamina = gambling.stamina - 1
            WHERE gambling.stamina > 0
            RETURNING coins, gems, stamina;",
            user_id,
            delta.coins,
            delta.gems,
            <DigRow as Stamina>::MAX_STAMINA - 1,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        #[expect(
            trivial_casts,
            reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
        )]
        let claimed = sqlx::query_scalar!(
            r#"INSERT INTO gambling_mine (user_id, mine_activity)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
                mine_activity = EXCLUDED.mine_activity
            WHERE gambling_mine.mine_activity = $3
            RETURNING user_id;"#,
            user_id,
            payout.collected_at.to_sqlx() as Timestamp,
            payout.since.to_sqlx() as Timestamp,
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        sqlx::query!(
            "INSERT INTO gambling_mine (user_id, coal, iron, gold, redstone, lapis, diamonds, emeralds)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id) DO UPDATE SET
                coal = gambling_mine.coal + $2,
                iron = gambling_mine.iron + $3,
                gold = gambling_mine.gold + $4,
                redstone = gambling_mine.redstone + $5,
                lapis = gambling_mine.lapis + $6,
                diamonds = gambling_mine.diamonds + $7,
                emeralds = gambling_mine.emeralds + $8;",
            user_id,
            delta.coal,
            delta.iron,
            delta.gold,
            delta.redstone,
            delta.lapis,
            delta.diamonds,
            delta.emeralds,
        )
        .execute(&mut *tx)
        .await?;

        let payout = if claimed { payout.coins } else { 0 };

        let coins = if payout == 0 {
            balance.coins
        } else {
            sqlx::query_scalar!(
                "UPDATE gambling SET coins = coins + $2
                WHERE user_id = $1
                RETURNING coins;",
                user_id,
                payout,
            )
            .fetch_one(&mut *tx)
            .await?
        };

        tx.commit().await?;

        Ok(Some(DigCommit {
            coins,
            gems: balance.gems,
            stamina: balance.stamina,
            payout,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DigDelta {
    pub coins: i64,
    pub gems: i64,
    pub coal: i64,
    pub iron: i64,
    pub gold: i64,
    pub redstone: i64,
    pub lapis: i64,
    pub diamonds: i64,
    pub emeralds: i64,
}

impl DigDelta {
    #[must_use]
    pub const fn between(before: &DigRow, after: &DigRow) -> Self {
        Self {
            coins: after.coins - before.coins,
            gems: after.gems - before.gems,
            coal: after.coal - before.coal,
            iron: after.iron - before.iron,
            gold: after.gold - before.gold,
            redstone: after.redstone - before.redstone,
            lapis: after.lapis - before.lapis,
            diamonds: after.diamonds - before.diamonds,
            emeralds: after.emeralds - before.emeralds,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct DigCommit {
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub payout: i64,
}

#[derive(Debug, Clone, FromRow)]
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
            user_id: as_i64(id.get()),
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
    pub async fn dig<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut row = DigManager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| DigRow::new(interaction.user.id));

        row.verify_work()?;

        let before = row.clone();

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
            let ore = as_i64(
                Binomial::new(as_u64(miners), (chance).min(1.0))
                    .map_err(|e| {
                        GamblingError::Internal(format!(
                            "Binomial params invalid: {e}"
                        ))
                    })?
                    .sample(&mut rng()),
            );

            let entry = resources.get_mut(resource).ok_or_else(|| {
                GamblingError::Internal(format!(
                    "resource key '{resource}' not in map"
                ))
            })?;
            *entry += match resource {
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

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(interaction.channel_id, &mut row, Event::Work(interaction.user.id))
            .await?;

        let payout =
            MinePayout::new(row.mine_amount()?, row.mine_activity.to_jiff());

        let delta = DigDelta::between(&before, &row);

        let committed =
            DigManager::commit_dig(pool, interaction.user.id, &delta, payout)
                .await?
                .ok_or_else(out_of_stamina)?;

        *row.coins_mut() = committed.coins;
        *row.gems_mut() = committed.gems;
        *row.stamina_mut() = committed.stamina;

        let mine_amount = committed.payout;
        let stamina = row.stamina_str();

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
                Ok(format!(
                    "{} `{}` {name}",
                    currency.emoji(&emojis)?,
                    amount.format()
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let coin = emojis.emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

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
