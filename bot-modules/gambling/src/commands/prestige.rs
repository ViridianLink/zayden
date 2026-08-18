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
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool};
use tracing::debug;
use zayden_core::{as_i64, message_metadata};

use crate::commands::inventory::InventoryManager;
use crate::common::shop::LOTTO_TICKET;
use crate::components::PrestigeCustomId;
use crate::stamina::MAX_STAMINA;
use crate::{
    Commands,
    GamblingError,
    MaxValues,
    Mining,
    Prestige,
    Result,
    START_AMOUNT,
};

pub struct PrestigeManager;

impl PrestigeManager {
    pub async fn miners(pool: &PgPool, id: UserId) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar!(
            "SELECT miners FROM gambling_mine WHERE user_id = $1;",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn row(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Option<PrestigeRow>> {
        sqlx::query_file_as!(
            PrestigeRow,
            "sql/PrestigeManager/row.sql",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn lotto(
        pool: &PgPool,
        tickets: i64,
        zayden_id: u64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/PrestigeManager/lotto.sql",
            as_i64(zayden_id),
            LOTTO_TICKET.id,
            tickets,
        )
        .execute(pool)
        .await
    }

    pub async fn save(
        pool: &PgPool,
        row: PrestigeRow,
        expected_prestige: i64,
        gems_awarded: i64,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;

        let mine = sqlx::query!(
            "UPDATE gambling_mine SET
                miners = $2,
                mines = $3,
                land = $4,
                countries = $5,
                continents = $6,
                planets = $7,
                solar_systems = $8,
                galaxies = $9,
                universes = $10,
                prestige = $11,
                coal = $12,
                iron = $13,
                gold = $14,
                redstone = $15,
                lapis = $16,
                diamonds = $17,
                emeralds = $18,
                tech = $19,
                utility = $20,
                production = $21
            WHERE user_id = $1 AND prestige = $22;",
            row.user_id,
            row.miners,
            row.mines,
            row.land,
            row.countries,
            row.continents,
            row.planets,
            row.solar_systems,
            row.galaxies,
            row.universes,
            row.prestige,
            row.coal,
            row.iron,
            row.gold,
            row.redstone,
            row.lapis,
            row.diamonds,
            row.emeralds,
            row.tech,
            row.utility,
            row.production,
            expected_prestige,
        )
        .execute(&mut *tx)
        .await?;

        if mine.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query!(
            "INSERT INTO gambling (user_id) VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING;",
            row.user_id,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE gambling SET
                coins = $2,
                gems = gems + $3,
                stamina = $4
            WHERE user_id = $1;",
            row.user_id,
            row.coins,
            gems_awarded,
            MAX_STAMINA,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM gambling_inventory
            WHERE user_id = $1;",
            row.user_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }
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

#[must_use]
pub fn miner_cap_without(rungs_above: u32) -> i64 {
    let per_rung = <PrestigeRow as MaxValues>::miners_per_mine();

    (0..rungs_above).fold(per_rung, |cap, _| per_rung * (cap + 1))
}

impl PrestigeRow {
    #[must_use]
    pub fn req_miners(&self) -> i64 {
        match self.prestige() {
            ..=4 => 1_000_000,
            5..=9 => 2_000_000,
            10..=14 => 20_000_000,
            _ => 200_000_000,
        }
    }

    #[must_use]
    pub const fn do_prestige(&mut self) -> i64 {
        self.prestige += 1;
        self.coins = START_AMOUNT;

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

        self.prestige
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
    pub async fn prestige(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row = PrestigeManager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_default();

        let req_miners = row.req_miners();

        if row.miners() < req_miners {
            return Err(GamblingError::NotEnoughMiners {
                required: req_miners,
                current: row.miners(),
            });
        }

        let embed = CreateEmbed::new().description("Are you sure you want to prestige your mine?\n\nPrestiging will **reset your mine, coins, items and resources**, but you'll unlock powerful upgrades!").colour(Colour::TEAL);

        let confirm = CreateButton::new(PrestigeCustomId::Confirm.as_str())
            .label("Confirm")
            .emoji('✅')
            .style(ButtonStyle::Secondary);
        let cancel = CreateButton::new(PrestigeCustomId::Cancel.as_str())
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

    pub async fn confirm_prestige(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        zayden_id: u64,
    ) -> Result<()> {
        let metadata = message_metadata(&interaction.message)?;

        if interaction.user != metadata.user {
            debug!(
                user_id = %interaction.user.id,
                owner_id = %metadata.user.id,
                "user does not own this prestige confirmation; ignoring"
            );
            return Ok(());
        }

        let Some(mut prestige_row) =
            PrestigeManager::row(pool, interaction.user.id).await?
        else {
            return Err(GamblingError::internal("user has no prestige row"));
        };

        if prestige_row.miners < prestige_row.req_miners() {
            return Err(GamblingError::internal(
                "not enough miners - component state is stale",
            ));
        }

        let inventory_row =
            InventoryManager::inventory_items(pool, interaction.user.id).await?;

        let lotto_tickets = inventory_row
            .0
            .iter()
            .find(|item| item.item_id == LOTTO_TICKET.id)
            .map(|item| item.quantity)
            .unwrap_or_default()
            .min(100_000);

        let expected_prestige = prestige_row.prestige;
        let gems_awarded = prestige_row.do_prestige();

        let applied = PrestigeManager::save(
            pool,
            prestige_row,
            expected_prestige,
            gems_awarded,
        )
        .await?;
        if !applied {
            return Err(GamblingError::internal(
                "prestige already completed - duplicate confirmation ignored",
            ));
        }

        PrestigeManager::lotto(pool, lotto_tickets, zayden_id).await?;

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
            debug!(
                user_id = %interaction.user.id,
                owner_id = %interaction.message.author.id,
                "user does not own this prestige message; ignoring cancel"
            );
            return Ok(());
        }

        interaction
            .message
            .delete(&ctx.http, Some("User canceled prestige"))
            .await?;

        Ok(())
    }
}
