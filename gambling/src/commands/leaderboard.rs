use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serenity::all::{
    CollectComponentInteractions, Colour, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse, Http, Mentionable, Message, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::{Database, Pool, prelude::FromRow};
use tokio::sync::RwLock;
use zayden_core::parse_options;
use zayden_core::{FormatNum, cache::GuildMembersCache};

use crate::shop::{EGGPLANT, LOTTO_TICKET};
use crate::{Coins, Gems, Result};

use super::Commands;

#[async_trait]
pub trait LeaderboardManager<Db: Database> {
    async fn networth(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn networth_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn coins(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn coins_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn gems(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn gems_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn eggplants(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn eggplants_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn lottotickets(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn lottotickets_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn higherlower(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn higherlower_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn weekly_higherlower(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        page_num: i64,
    ) -> sqlx::Result<Vec<LeaderboardRow>>;

    async fn weekly_higherlower_row_number(
        pool: &Pool<Db>,
        global: bool,
        users: &[i64],
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;
}

#[derive(FromRow)]
pub struct NetworthRow {
    pub id: i64,
    pub networth: Option<i64>,
}

#[derive(FromRow)]
pub struct CoinsRow {
    pub id: i64,
    pub coins: i64,
}

impl Coins for CoinsRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

#[derive(FromRow)]
pub struct GemsRow {
    pub id: i64,
    pub gems: i64,
}

impl Gems for GemsRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

#[derive(FromRow)]
pub struct EggplantsRow {
    pub user_id: i64,
    pub quantity: i64,
}

#[derive(FromRow)]
pub struct LottoTicketRow {
    pub user_id: i64,
    pub quantity: i64,
}

#[derive(FromRow)]
pub struct HigherLowerRow {
    pub user_id: i64,
    pub higher_or_lower_score: i32,
}

#[derive(FromRow)]
pub struct WeeklyHigherLowerRow {
    pub user_id: i64,
    pub weekly_higher_or_lower_score: i32,
}

impl Commands {
    pub async fn leaderboard<
        Data: GuildMembersCache,
        Db: Database,
        Manager: LeaderboardManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let mut options = parse_options(options);

        let ResolvedValue::String(leaderboard) = options.remove("leaderboard").unwrap() else {
            unreachable!("leaderboard option is required")
        };

        let global = match options.remove("global") {
            Some(ResolvedValue::Boolean(global)) => global,
            _ => false,
        };

        let users = if !global {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            let users = data
                .get()
                .get(&interaction.guild_id.unwrap())
                .unwrap()
                .iter()
                .map(|id| id.get() as i64)
                .collect::<Vec<_>>();
            Some(users)
        } else {
            None
        };

        let rows = get_rows::<Db, Manager>(leaderboard, pool, users.as_deref(), 1).await;

        let desc = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| row.as_desc(i))
            .collect::<Vec<_>>()
            .join("\n\n");

        let embed = CreateEmbed::new()
            .title(format!("🏁 Leaderboard ({leaderboard})"))
            .description(desc)
            .footer(CreateEmbedFooter::new("Page 1"))
            .colour(Colour::TEAL);

        let mut response = EditInteractionResponse::new()
            .embed(embed)
            .button(CreateButton::new("leaderboard_previous").label("<"));

        if get_row_number::<Db, Manager>(leaderboard, pool, users.as_deref(), interaction.user.id)
            .await
            .is_some()
        {
            response = response.button(CreateButton::new("leaderboard_user").emoji('🎯'));
        }

        let msg = interaction
            .edit_response(
                &ctx.http,
                response.button(CreateButton::new("leaderboard_next").label(">")),
            )
            .await
            .unwrap();

        let mut stream = msg
            .id
            .collect_component_interactions(ctx)
            .timeout(Duration::from_secs(120))
            .stream();

        while let Some(component) = stream.next().await {
            run_component::<Db, Manager>(&ctx.http, pool, users.as_deref(), &msg, component)
                .await?;
        }

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().components(Vec::new()),
            )
            .await?;

        Ok(())
    }

    pub fn register_leaderboard<'a>() -> CreateCommand<'a> {
        CreateCommand::new("leaderboard")
            .description("The server leaderboard")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "leaderboard",
                    "The leaderboard to choose",
                )
                .required(true)
                .add_string_choice("Net Worth", "networth")
                .add_string_choice("Coins", "coins")
                .add_string_choice("Gems", "gems")
                .add_string_choice(EGGPLANT.name, "eggplants")
                .add_string_choice(LOTTO_TICKET.name, "lottotickets")
                .add_string_choice("Higher or Lower", "higherlower")
                .add_string_choice("Weekly Higher or Lower", "weekly_higherlower"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "global",
                "Whether to show global scores",
            ))
    }
}

async fn run_component<Db: Database, Manager: LeaderboardManager<Db>>(
    http: &Http,
    pool: &Pool<Db>,
    users: Option<&[i64]>,
    msg: &Message,
    interaction: ComponentInteraction,
) -> Result<()> {
    let custom_id = interaction
        .data
        .custom_id
        .strip_prefix("leaderboard_")
        .unwrap();

    let embed = msg.embeds.first().unwrap();

    let leaderboard = embed
        .title
        .as_ref()
        .unwrap()
        .strip_prefix("🏁 Leaderboard (")
        .unwrap()
        .strip_suffix(")")
        .unwrap();

    let mut page_number: i64 = embed
        .footer
        .as_ref()
        .unwrap()
        .text
        .strip_prefix("Page ")
        .unwrap()
        .parse()
        .unwrap();

    match custom_id {
        "previous" => {
            page_number = (page_number - 1).max(1);
        }
        "user" => {
            let row_num =
                get_row_number::<Db, Manager>(leaderboard, pool, users, interaction.user.id)
                    .await
                    .unwrap();
            page_number = row_num / 10 + 1;
        }
        "next" => {
            page_number += 1;
        }
        _ => unreachable!("Invalid custom id"),
    };

    let rows = get_rows::<Db, Manager>(leaderboard, pool, users, page_number).await;

    if rows.is_empty() {
        return Ok(());
    }

    let desc = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| row.as_desc(i + (page_number as usize - 1) * 10))
        .collect::<Vec<_>>()
        .join("\n\n");

    let embed = CreateEmbed::from(embed.clone())
        .footer(CreateEmbedFooter::new(format!("Page {page_number}")))
        .description(desc);

    interaction
        .create_response(
            http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .unwrap();

    Ok(())
}

pub enum LeaderboardRow {
    NetWorth(NetworthRow),
    Coins(CoinsRow),
    Gems(GemsRow),
    Eggplants(EggplantsRow),
    LottoTickets(LottoTicketRow),
    HigherLower(HigherLowerRow),
    WeeklyHigherLower(WeeklyHigherLowerRow),
}

impl LeaderboardRow {
    pub fn user_id(&self) -> UserId {
        match self {
            Self::NetWorth(row) => UserId::new(row.id as u64),
            Self::Coins(row) => UserId::new(row.id as u64),
            Self::Gems(row) => UserId::new(row.id as u64),
            Self::Eggplants(row) => UserId::new(row.user_id as u64),
            Self::LottoTickets(row) => UserId::new(row.user_id as u64),
            Self::HigherLower(row) => UserId::new(row.user_id as u64),
            Self::WeeklyHigherLower(row) => UserId::new(row.user_id as u64),
        }
    }

    pub fn as_desc(&self, i: usize) -> String {
        let place = if i == 0 {
            "🥇".to_string()
        } else if i == 1 {
            "🥈".to_string()
        } else if i == 2 {
            "🥉".to_string()
        } else {
            format!("#{}", i + 1)
        };

        let data = match self {
            Self::NetWorth(row) => row.networth.unwrap_or_default().format(),
            Self::Coins(row) => row.coins_str(),
            Self::Gems(row) => row.gems_str(),
            Self::Eggplants(row) => format!("{} {}", row.quantity.format(), EGGPLANT.emoji()),
            Self::LottoTickets(row) => {
                format!("{} {}", row.quantity.format(), LOTTO_TICKET.emoji())
            }
            Self::HigherLower(row) => row.higher_or_lower_score.to_string(),
            Self::WeeklyHigherLower(row) => row.weekly_higher_or_lower_score.to_string(),
        };

        format!("{place} - {} - {data}", self.user_id().mention())
    }
}

async fn get_rows<Db: Database, Manager: LeaderboardManager<Db>>(
    leaderboard: &str,
    pool: &Pool<Db>,
    users: Option<&[i64]>,
    page_num: i64,
) -> Vec<LeaderboardRow> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "networth" => Manager::networth(pool, global, users, page_num)
            .await
            .unwrap(),
        "coins" => Manager::coins(pool, global, users, page_num).await.unwrap(),
        "gems" => Manager::gems(pool, global, users, page_num).await.unwrap(),
        "eggplants" => Manager::eggplants(pool, global, users, page_num)
            .await
            .unwrap(),
        "lottotickets" => Manager::lottotickets(pool, global, users, page_num)
            .await
            .unwrap(),
        "higherlower" => Manager::higherlower(pool, global, users, page_num)
            .await
            .unwrap(),
        "weekly_higherlower" => Manager::weekly_higherlower(pool, global, users, page_num)
            .await
            .unwrap(),
        _ => unreachable!("Invalid leaderboard option"),
    }
}

async fn get_row_number<Db: Database, Manager: LeaderboardManager<Db>>(
    leaderboard: &str,
    pool: &Pool<Db>,
    users: Option<&[i64]>,
    user: UserId,
) -> Option<i64> {
    let global = users.is_none();
    let users = users.unwrap_or_default();

    match leaderboard {
        "coins" => Manager::coins_row_number(pool, global, users, user)
            .await
            .unwrap(),
        "gems" => Manager::gems_row_number(pool, global, users, user)
            .await
            .unwrap(),
        "eggplants" => Manager::eggplants_row_number(pool, global, users, user)
            .await
            .unwrap(),
        "lottotickets" => Manager::lottotickets_row_number(pool, global, users, user)
            .await
            .unwrap(),
        "higherlower" => Manager::higherlower_row_number(pool, global, users, user)
            .await
            .unwrap(),
        "weekly_higherlower" => Manager::weekly_higherlower_row_number(pool, global, users, user)
            .await
            .unwrap(),
        _ => None,
    }
}
