use serenity::all::{
    ButtonStyle, CommandInteraction, Context, CreateButton, CreateCommand, EditInteractionResponse,
};
use sqlx::{Database, Pool};
use zayden_core::cache::GuildMembersCache;

use crate::common::levels::create_embed;
use crate::{Levels, LevelsManager};

impl Levels {
    pub async fn run<Data: GuildMembersCache, Db: Database, Manager: LevelsManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) {
        interaction.defer(&ctx.http).await.unwrap();

        let embed =
            create_embed::<Data, Db, Manager>(ctx, pool, interaction.guild_id.unwrap(), 1).await;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .button(
                        CreateButton::new("levels_previous")
                            .label("<")
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new("levels_user")
                            .emoji('🎯')
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new("levels_next")
                            .label(">")
                            .style(ButtonStyle::Secondary),
                    ),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("levels").description("Get the leaderboard")
    }
}
