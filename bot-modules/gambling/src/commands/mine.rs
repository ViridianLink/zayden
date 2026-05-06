use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, EditInteractionResponse, UserId,
};
use sqlx::{Database, FromRow, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum};

use crate::{MaxValues, MineHourly, Mining, Prestige, Result};

#[async_trait]
pub trait MineManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<MineRow>>;
}

#[derive(Default, FromRow)]
pub struct MineRow {
    pub miners: i64,
    pub mines: i64,
    pub land: i64,
    pub countries: i64,
    pub continents: i64,
    pub planets: i64,
    pub solar_systems: i64,
    pub galaxies: i64,
    pub universes: i64,
    pub prestige: i64,
}

impl Mining for MineRow {
    fn miners(&self) -> i64 {
        self.miners
    }

    fn mines(&self) -> i64 {
        self.mines
    }

    fn land(&self) -> i64 {
        self.land
    }

    fn countries(&self) -> i64 {
        self.countries
    }

    fn continents(&self) -> i64 {
        self.continents
    }

    fn planets(&self) -> i64 {
        self.planets
    }

    fn solar_systems(&self) -> i64 {
        self.solar_systems
    }

    fn galaxies(&self) -> i64 {
        self.galaxies
    }

    fn universes(&self) -> i64 {
        self.universes
    }

    fn tech(&self) -> i64 {
        unimplemented!()
    }

    fn utility(&self) -> i64 {
        unimplemented!()
    }

    fn production(&self) -> i64 {
        unimplemented!()
    }

    fn coal(&self) -> i64 {
        unimplemented!()
    }

    fn iron(&self) -> i64 {
        unimplemented!()
    }

    fn gold(&self) -> i64 {
        unimplemented!()
    }

    fn redstone(&self) -> i64 {
        unimplemented!()
    }

    fn lapis(&self) -> i64 {
        unimplemented!()
    }

    fn diamonds(&self) -> i64 {
        unimplemented!()
    }

    fn emeralds(&self) -> i64 {
        unimplemented!()
    }
}

impl Prestige for MineRow {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl MineHourly for MineRow {
    fn miners(&self) -> i64 {
        self.miners
    }
}

use super::Commands;

impl Commands {
    pub async fn mine<Data: EmojiCacheData, Db: Database, Manager: MineManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row = Manager::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_default();

        let coin = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis().emoji("heads").unwrap()
        };

        let embed = CreateEmbed::new()
            .field(
                "Mine Income",
                format!("{} <:coin:{coin}> / hour", row.hourly().format()),
                false,
            )
            .field("Units", row.units(), false);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_mine<'a>() -> CreateCommand<'a> {
        CreateCommand::new("mine").description("Show the details of your mine")
    }
}
