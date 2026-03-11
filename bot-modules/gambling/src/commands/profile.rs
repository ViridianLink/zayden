use async_trait::async_trait;
use jiff::Timestamp;
use levels::{LevelsRow, level_up_xp};
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::{
    Coins, GamblingItems, Gems, MaxBet, Prestige, Result, ShopItem,
    commands::inventory::InventoryManager,
};

use super::Commands;

#[async_trait]
pub trait ProfileManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send)
    -> sqlx::Result<Option<ProfileRow>>;
}

#[derive(Default)]
pub struct ProfileRow {
    pub coins: i64,
    pub gems: i64,
    pub xp: Option<i32>,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl ProfileRow {
    pub fn into_embed<'a>(self, inventory: GamblingItems, emojis: &EmojiCache) -> CreateEmbed<'a> {
        let mut betting_max = self.max_bet_str();
        if self.prestige() != 0 {
            betting_max.push_str(&format!("\n(Prestige Boost: +{}%)", 10 * self.prestige()));
        }

        let loot_str = if inventory.0.is_empty() {
            String::from("You've got no loot, not even a 🥄")
        } else {
            inventory
                .0
                .iter()
                .filter(|item| item.quantity > 0)
                .map(|inv| (inv, ShopItem::from(inv)))
                .map(|(inv, item)| {
                    format!("{} {} {}s", item.emoji(emojis), inv.quantity, item.name)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let coin = emojis.emoji("heads").unwrap();

        CreateEmbed::new()
            .field(format!("Coins <:coin:{coin}>"), self.coins_str(), false)
            .field("Gems 💎", self.gems_str(), false)
            .field(
                format!("Level {}", LevelsRow::level(&self).format()),
                format!(
                    "{} / {} xp",
                    self.xp().format(),
                    level_up_xp(LevelsRow::level(&self)).format()
                ),
                false,
            )
            .field("Betting Maximum", betting_max, false)
            .field("Loot", loot_str, false)
            .colour(Colour::TEAL)
    }
}

impl Coins for ProfileRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for ProfileRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl LevelsRow for ProfileRow {
    fn user_id(&self) -> UserId {
        unimplemented!()
    }

    fn xp(&self) -> i32 {
        self.xp.unwrap_or_default()
    }

    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }

    fn total_xp(&self) -> i64 {
        unimplemented!()
    }

    fn message_count(&self) -> i64 {
        unimplemented!()
    }

    fn last_xp(&self) -> Timestamp {
        unimplemented!()
    }
}

impl Prestige for ProfileRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for ProfileRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl Commands {
    pub async fn profile<
        Data: EmojiCacheData,
        Db: Database,
        Manager: ProfileManager<Db> + InventoryManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let user = match options.pop() {
            Some(option) => {
                let ResolvedValue::User(user, _) = option.value else {
                    unreachable!("value must be a user")
                };
                user
            }
            None => &interaction.user,
        };

        let row = Manager::row(pool, user.id).await?.unwrap_or_default();
        let inventory_row = Manager::inventory_items(pool, user.id).await.unwrap();

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut embed = row
            .into_embed(inventory_row, &emojis)
            .title(user.display_name());

        if let Some(avatar) = user.avatar_url() {
            embed = embed.thumbnail(avatar);
        }

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_profile<'a>() -> CreateCommand<'a> {
        CreateCommand::new("profile")
            .description("Show your coins, level and items")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user's profile to show",
            ))
    }
}
