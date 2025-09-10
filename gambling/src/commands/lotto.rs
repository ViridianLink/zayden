use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, EditInteractionResponse,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum};

use crate::shop::LOTTO_TICKET;
use crate::{Commands, GamblingManager, Lotto, LottoManager, LottoRow, Result, jackpot};

impl Commands {
    pub async fn lotto<
        Data: EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        LottoHandler: LottoManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let mut tx = pool.begin().await.unwrap();

        let total_tickets = LottoHandler::total_tickets(&mut tx).await.unwrap();

        let row = match LottoHandler::row(&mut tx, interaction.user.id)
            .await
            .unwrap()
        {
            Some(row) => row,
            None => LottoRow::new(interaction.user.id),
        };

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let lotto_emoji = LOTTO_TICKET.emoji(&emojis);

        let timestamp = {
            Lotto::cron_job::<Data, Db, GamblingHandler, LottoHandler>()
                .schedule
                .upcoming(chrono::Utc)
                .next()
                .unwrap_or_default()
                .timestamp()
        };

        let coin = emojis.emoji("heads").unwrap();

        let embed = CreateEmbed::new()
            .title(format!(
                "<:coin:{coin}> <:coin:{coin}> Lottery!! <:coin:{coin}> <:coin:{coin}>"
            ))
            .description(format!("Draws are at <t:{timestamp}:F>"))
            .field(
                "Tickets Bought",
                format!("{} {lotto_emoji}", total_tickets.format()),
                false,
            )
            .field(
                "Jackpot Value",
                format!("{} <:coin:{coin}>", jackpot(total_tickets).format()),
                false,
            )
            .field(
                "Your Tickets",
                format!("{} {lotto_emoji}", row.quantity().format()),
                false,
            );

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_lotto<'a>() -> CreateCommand<'a> {
        CreateCommand::new("lotto").description("Show the lottery information")
    }
}
