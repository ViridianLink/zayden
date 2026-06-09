use async_trait::async_trait;
use serenity::all::{
    ButtonStyle,
    Colour,
    CommandInteraction,
    ComponentInteraction,
    Context,
    CreateButton,
    CreateCommand,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EditInteractionResponse,
    UserId,
};
use sqlx::{Database, FromRow, Pool};
use zayden_core::message_metadata;

use crate::commands::inventory::InventoryManager;
use crate::common::shop::LOTTO_TICKET;
use crate::{
    Commands,
    GamblingError,
    MaxValues,
    Mining,
    Prestige,
    Result,
    START_AMOUNT,
};

#[async_trait]
pub trait PrestigeManager<Db: Database> {
    async fn miners(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>>;

    async fn row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<PrestigeRow>>;

    async fn lotto(pool: &Pool<Db>, tickets: i64) -> sqlx::Result<Db::QueryResult>;

    async fn save(
        pool: &Pool<Db>,
        row: PrestigeRow,
    ) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow, Default)]
pub struct PrestigeRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i64,
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

impl PrestigeRow {
    #[must_use]
    pub fn req_miners(&self) -> i64 {
        let prestige = self.prestige();

        let mut required_miners = Self::plants_per_solar_system()
            * Self::continents_per_plant()
            * Self::countries_per_continent()
            * Self::land_per_country()
            * Self::mines_per_land()
            * Self::miners_per_mine();

        if prestige >= 5 {
            required_miners *= Self::solar_system_per_galaxies();
        }
        if prestige >= 10 {
            required_miners *= Self::galaxies_per_universe();
        }
        if prestige >= 15 {
            required_miners *= self.universes;
        }

        required_miners
    }

    pub const fn do_prestige(&mut self) {
        self.prestige += 1;
        self.coins = START_AMOUNT;
        self.gems += self.prestige;
        self.stamina = 3;

        self.miners = 0;
        self.mines = 0;
        self.land = 0;
        self.countries = 0;
        self.continents = 0;
        self.planets = 0;
        self.solar_systems = 0;
        self.galaxies = 0;
        self.universes = 0;
        self.coal = 0;
        self.iron = 0;
        self.gold = 0;
        self.redstone = 0;
        self.lapis = 0;
        self.diamonds = 0;
        self.emeralds = 0;
        self.tech = 0;
        self.utility = 0;
        self.production = 0;
    }
}

impl Mining for PrestigeRow {
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
        self.tech
    }

    fn utility(&self) -> i64 {
        self.utility
    }

    fn production(&self) -> i64 {
        self.production
    }

    fn coal(&self) -> i64 {
        self.coal
    }

    fn iron(&self) -> i64 {
        self.iron
    }

    fn gold(&self) -> i64 {
        self.gold
    }

    fn redstone(&self) -> i64 {
        self.redstone
    }

    fn lapis(&self) -> i64 {
        self.lapis
    }

    fn diamonds(&self) -> i64 {
        self.diamonds
    }

    fn emeralds(&self) -> i64 {
        self.emeralds
    }
}

impl Prestige for PrestigeRow {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl Commands {
    pub async fn prestige<Db: Database, Manager: PrestigeManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row = Manager::row(pool, interaction.user.id).await?.unwrap_or_default();

        let req_miners = row.req_miners();

        if row.miners() < req_miners {
            return Err(GamblingError::NotEnoughMiners {
                required: req_miners,
                current: row.miners(),
            });
        }

        let embed = CreateEmbed::new().description("Are you sure you want to prestige your mine?\n\nPrestiging will **reset your mine, coins, items and resources**, but you'll unlock powerful upgrades!").colour(Colour::TEAL);

        let confirm = CreateButton::new("prestige_confirm")
            .label("Confirm")
            .emoji('✅')
            .style(ButtonStyle::Secondary);
        let cancel = CreateButton::new("prestige_cancel")
            .label("Cancel")
            .emoji('❌')
            .style(ButtonStyle::Secondary);

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .button(confirm)
                    .button(cancel),
            )
            .await?;

        Ok(())
    }

    pub fn register_prestige<'a>() -> CreateCommand<'a> {
        CreateCommand::new("prestige")
            .description("Prestige your mine or casino to get unique rewards!")
    }

    pub async fn confirm_prestige<
        Db: Database,
        Manager: PrestigeManager<Db> + InventoryManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let metadata = message_metadata(&interaction.message)?;

        if interaction.user != metadata.user {
            return Ok(());
        }

        let Some(mut prestige_row) = Manager::row(pool, interaction.user.id).await?
        else {
            return Err(GamblingError::internal("user has no prestige row"));
        };

        if prestige_row.miners < prestige_row.req_miners() {
            return Err(GamblingError::internal(
                "not enough miners — component state is stale",
            ));
        }

        let mut inventory_row =
            Manager::inventory_items(pool, interaction.user.id).await?;

        Manager::lotto(
            pool,
            inventory_row
                .0
                .iter()
                .find(|item| item.item_id == LOTTO_TICKET.id)
                .map(|item| item.quantity)
                .unwrap_or_default()
                .min(100_000),
        )
        .await?;
        prestige_row.do_prestige();
        inventory_row.do_prestige();

        Manager::save(pool, prestige_row).await?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Done (message wip)")
                        .embeds(Vec::new())
                        .components(Vec::new()),
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn cancel_prestige(
        ctx: &Context,
        interaction: &ComponentInteraction,
    ) -> Result<()> {
        if interaction.user.id != interaction.message.author.id {
            return Ok(());
        }

        interaction
            .message
            .delete(&ctx.http, Some("User canceled prestige"))
            .await?;

        Ok(())
    }
}
