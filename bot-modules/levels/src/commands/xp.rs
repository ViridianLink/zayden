use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Http,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::parse_options;

use crate::{LevelsRow, Result, XpRow};

pub struct Xp;

impl Xp {
    pub async fn xp(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        let mut options = parse_options(options);

        match options.remove("ephemeral") {
            Some(ResolvedValue::Boolean(true)) => {
                interaction.defer_ephemeral(http).await?;
            },
            _ => interaction.defer(http).await?,
        }

        let row = XpRow::get(pool, interaction.user.id).await?.unwrap_or_default();

        let embed = CreateEmbed::default().title("XP").description(format!(
            "Current XP: {}\nLevel: {}\nTotal XP: {}",
            row.xp(),
            row.level(),
            row.total_xp()
        ));

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("xp").description("Get your current xp").add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "ephemeral",
                "Whether the response should be ephemeral",
            ),
        )
    }
}
