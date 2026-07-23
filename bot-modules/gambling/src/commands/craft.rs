use async_trait::async_trait;
use serenity::all::{
    Colour,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Http,
    ResolvedOption,
    UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{
    EmojiCache,
    EmojiCacheData,
    FormatNum,
    as_i64,
    optional_option,
    parse_options,
};

use super::Commands;
use crate::shop::ShopCurrency;
use crate::{GamblingError, Result};

#[async_trait]
pub trait CraftManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: UserId) -> sqlx::Result<Option<CraftRow>>;

    async fn save(pool: &Pool<Db>, row: CraftRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct CraftRow {
    pub user_id: i64,
    pub coal: i64,
    pub iron: i64,
    pub gold: i64,
    pub redstone: i64,
    pub lapis: i64,
    pub diamonds: i64,
    pub emeralds: i64,
    pub tech: i64,
    pub utility: i64,
    pub production: i64,
}

impl CraftRow {
    #[must_use]
    pub const fn new(id: UserId) -> Self {
        Self {
            user_id: as_i64(id.get()),
            coal: 0,
            iron: 0,
            gold: 0,
            redstone: 0,
            lapis: 0,
            diamonds: 0,
            emeralds: 0,
            tech: 0,
            utility: 0,
            production: 0,
        }
    }
}

impl Commands {
    pub async fn craft<
        Data: EmojiCacheData,
        Db: Database,
        Manager: CraftManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut row = Manager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| CraftRow::new(interaction.user.id));

        let mut options = parse_options(options);

        let kind: &str = match optional_option(&mut options, "type") {
            Some(k) => k,
            None => {
                menu(&ctx.http, interaction, &emojis, row).await?;

                return Ok(());
            },
        };

        let amount: i64 = optional_option(&mut options, "amount").unwrap_or(1);

        if amount.is_negative() {
            return Err(GamblingError::NegativeAmount);
        }

        if amount == 0 {
            return Err(GamblingError::ZeroAmount);
        }

        let item: ShopCurrency =
            kind.parse().map_err(|_e| GamblingError::InvalidAmount)?;

        let costs = item
            .craft_req(&emojis)
            .into_iter()
            .flatten()
            .map(|(currency, cost)| (currency, i64::from(cost) * amount))
            .collect::<Vec<_>>();

        for (currency, cost) in costs {
            let fund = match currency {
                ShopCurrency::Coal => &mut row.coal,
                ShopCurrency::Iron => &mut row.iron,
                ShopCurrency::Gold => &mut row.gold,
                ShopCurrency::Redstone => &mut row.redstone,
                ShopCurrency::Lapis => &mut row.lapis,
                ShopCurrency::Diamonds => &mut row.diamonds,
                ShopCurrency::Emeralds => &mut row.emeralds,
                ShopCurrency::Coins
                | ShopCurrency::Gems
                | ShopCurrency::Tech
                | ShopCurrency::Utility
                | ShopCurrency::Production => {
                    return Err(GamblingError::InvalidAmount);
                },
            };

            *fund -= cost;
            if *fund < 0 {
                return Err(GamblingError::InsufficientFunds {
                    required: fund.abs(),
                    currency,
                });
            }
        }

        let quantity = match item {
            ShopCurrency::Tech => {
                row.tech += amount;
                row.tech
            },
            ShopCurrency::Utility => {
                row.utility += amount;
                row.utility
            },
            ShopCurrency::Production => {
                row.production += amount;
                row.production
            },
            ShopCurrency::Coins
            | ShopCurrency::Gems
            | ShopCurrency::Coal
            | ShopCurrency::Iron
            | ShopCurrency::Gold
            | ShopCurrency::Redstone
            | ShopCurrency::Lapis
            | ShopCurrency::Diamonds
            | ShopCurrency::Emeralds => return Err(GamblingError::InvalidAmount),
        };

        Manager::save(pool, row).await?;

        let embed = CreateEmbed::new()
            .description(format!(
                "Crafted {0} `{1}` {item:?}s\nYou now  have {0} `{2}` {item:?}s",
                item.emoji(&emojis)?,
                amount.format(),
                quantity.format()
            ))
            .colour(Colour::ORANGE);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_craft<'a>() -> CreateCommand<'a> {
        CreateCommand::new("craft")
            .description("Craft packs to buy mining units")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "type",
                    "The type of pack to craft",
                )
                .add_string_choice("Tech Pack", "tech")
                .add_string_choice("Utility Pack", "utility")
                .add_string_choice("Production Pack", "production"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "amount",
                "The amount to craft",
            ))
    }
}

async fn menu(
    http: &Http,
    interaction: &CommandInteraction,
    emojis: &EmojiCache,
    row: CraftRow,
) -> Result<()> {
    let mut sections = Vec::new();

    for item in [ShopCurrency::Tech, ShopCurrency::Utility, ShopCurrency::Production]
    {
        let owned = match item {
            ShopCurrency::Tech => row.tech.format(),
            ShopCurrency::Utility => row.utility.format(),
            ShopCurrency::Production => row.production.format(),
            ShopCurrency::Coins
            | ShopCurrency::Gems
            | ShopCurrency::Coal
            | ShopCurrency::Iron
            | ShopCurrency::Gold
            | ShopCurrency::Redstone
            | ShopCurrency::Lapis
            | ShopCurrency::Diamonds
            | ShopCurrency::Emeralds => String::new(),
        };

        let mut req_lines = Vec::new();
        for (currency, cost) in item.craft_req(emojis).into_iter().flatten() {
            let inv = match currency {
                ShopCurrency::Coal => row.coal.format(),
                ShopCurrency::Iron => row.iron.format(),
                ShopCurrency::Gold => row.gold.format(),
                ShopCurrency::Redstone => row.redstone.format(),
                ShopCurrency::Lapis => row.lapis.format(),
                ShopCurrency::Diamonds => row.diamonds.format(),
                ShopCurrency::Emeralds => row.emeralds.format(),
                ShopCurrency::Coins
                | ShopCurrency::Gems
                | ShopCurrency::Tech
                | ShopCurrency::Utility
                | ShopCurrency::Production => String::new(),
            };
            req_lines
                .push(format!("(`{inv}`) `{cost}` {}", currency.emoji(emojis)?));
        }

        sections.push(format!(
            "{} **{item:?}**\nOwned: `{owned}`\n{}",
            item.emoji(emojis)?,
            req_lines.join("\n")
        ));
    }

    let mut desc = sections.join("\n\n");

    desc.push_str("\n------------------\n`/craft <id> <amount>`");

    let embed = CreateEmbed::new()
        .title("Craftable Items")
        .description(desc)
        .colour(Colour::ORANGE);

    interaction
        .edit_response(http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
