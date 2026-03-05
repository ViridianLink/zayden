use async_trait::async_trait;
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse, Http, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum, parse_options};

use crate::shop::ShopCurrency;
use crate::{Error, Result};

use super::Commands;

#[async_trait]
pub trait CraftManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<CraftRow>>;

    async fn save(pool: &Pool<Db>, row: CraftRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct CraftRow {
    pub id: i64,
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
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
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
    pub async fn craft<Data: EmojiCacheData, Db: Database, Manager: CraftManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut row = Manager::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| CraftRow::new(interaction.user.id));

        let mut options = parse_options(options);

        if !options.contains_key("type") {
            menu(&ctx.http, interaction, &emojis, row).await;
            return Ok(());
        }

        let Some(ResolvedValue::String(kind)) = options.remove("type") else {
            unreachable!("Type must be present")
        };

        let amount = match options.remove("amount") {
            Some(ResolvedValue::Integer(amount)) => amount,
            _ => 1,
        };

        if amount.is_negative() {
            return Err(Error::NegativeAmount);
        }

        if amount == 0 {
            return Err(Error::ZeroAmount);
        }

        let item: ShopCurrency = kind.parse().unwrap();

        let costs = item
            .craft_req(&emojis)
            .into_iter()
            .flatten()
            .map(|(currency, cost)| (currency, cost as i64 * amount))
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
                c => unreachable!("Invalid crafting currency: {c:?}"),
            };

            *fund -= cost;
            if *fund < 0 {
                return Err(Error::InsufficientFunds {
                    required: fund.abs(),
                    currency,
                });
            }
        }

        let quantity = match item {
            ShopCurrency::Tech => {
                row.tech += amount;
                row.tech
            }
            ShopCurrency::Utility => {
                row.utility += amount;
                row.utility
            }
            ShopCurrency::Production => {
                row.production += amount;
                row.production
            }
            c => unreachable!("Invalid item: {c:?}"),
        };

        Manager::save(pool, row).await.unwrap();

        let embed = CreateEmbed::new()
            .description(format!(
                "Crafted {0} `{1}` {item:?}s\nYou now  have {0} `{2}` {item:?}s",
                item.emoji(&emojis),
                amount.format(),
                quantity.format()
            ))
            .colour(Colour::ORANGE);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

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

async fn menu(http: &Http, interaction: &CommandInteraction, emojis: &EmojiCache, row: CraftRow) {
    let mut desc = [
        ShopCurrency::Tech,
        ShopCurrency::Utility,
        ShopCurrency::Production,
    ]
    .into_iter()
    .map(|item| match item {
        ShopCurrency::Tech => (item, row.tech.format()),
        ShopCurrency::Utility => (item, row.utility.format()),
        ShopCurrency::Production => (item, row.production.format()),
        _ => unreachable!(),
    })
    .map(|(item, owned)| {
        format!(
            "{} **{item:?}**\nOwned: `{owned}`\n{}",
            item.emoji(emojis),
            item.craft_req(emojis)
                .into_iter()
                .flatten()
                .map(|(currency, cost)| {
                    match currency {
                        ShopCurrency::Coal => (currency, cost, row.coal.format()),
                        ShopCurrency::Iron => (currency, cost, row.iron.format()),
                        ShopCurrency::Gold => (currency, cost, row.gold.format()),
                        ShopCurrency::Redstone => (currency, cost, row.redstone.format()),
                        ShopCurrency::Lapis => (currency, cost, row.lapis.format()),
                        ShopCurrency::Diamonds => (currency, cost, row.diamonds.format()),
                        ShopCurrency::Emeralds => (currency, cost, row.emeralds.format()),
                        _ => unreachable!("Invalid shop currency"),
                    }
                })
                .map(|(currency, cost, owned)| format!(
                    "(`{owned}`) `{cost}` {}",
                    currency.emoji(emojis)
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
    .collect::<Vec<_>>()
    .join("\n\n");

    desc.push_str("\n------------------\n`/craft <id> <amount>`");

    let embed = CreateEmbed::new()
        .title("Craftable Items")
        .description(desc)
        .colour(Colour::ORANGE);

    interaction
        .edit_response(http, EditInteractionResponse::new().embed(embed))
        .await
        .unwrap();
}
