use serenity::all::{
    CommandInteraction, CreateCommand, CreateEmbed, EditInteractionResponse, Http,
};
use sqlx::{Database, Pool};
use zayden_core::FormatNum;

use crate::shop::LOTTO_TICKET;
use crate::{COIN, Commands, GamblingManager, Lotto, LottoManager, LottoRow, Result, jackpot};

impl Commands {
    pub async fn lotto<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        LottoHandler: LottoManager<Db>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await.unwrap();

        let mut tx = pool.begin().await.unwrap();

        let total_tickets = LottoHandler::total_tickets(&mut tx).await.unwrap();

        let row = match LottoHandler::row(&mut tx, interaction.user.id)
            .await
            .unwrap()
        {
            Some(row) => row,
            None => LottoRow::new(interaction.user.id),
        };

        let lotto_emoji = LOTTO_TICKET.emoji();

        let timestamp = {
            Lotto::cron_job::<Db, GamblingHandler, LottoHandler>()
                .schedule
                .upcoming(chrono::Utc)
                .next()
                .unwrap_or_default()
                .timestamp()
        };

        let embed = CreateEmbed::new()
            .title(format!(
                "<:coin:{COIN}> <:coin:{COIN}> Lottery!! <:coin:{COIN}> <:coin:{COIN}>"
            ))
            .description(format!("Draws are at <t:{timestamp}:F>"))
            .field(
                "Tickets Bought",
                format!("{} {lotto_emoji}", total_tickets.format()),
                false,
            )
            .field(
                "Jackpot Value",
                format!("{} <:coin:{COIN}>", jackpot(total_tickets).format()),
                false,
            )
            .field(
                "Your Tickets",
                format!("{} {lotto_emoji}", row.quantity().format()),
                false,
            );

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_lotto<'a>() -> CreateCommand<'a> {
        CreateCommand::new("lotto").description("Show the lottery information")
    }
}
