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
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{LevelsManager, LevelsRow, Result, level_up_xp};

pub struct Rank;

impl Rank {
    pub async fn rank<Db: Database, Manager: LevelsManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let mut options = parse_options(options);

        match options.remove("ephemeral") {
            Some(ResolvedValue::Boolean(true)) => {
                interaction.defer_ephemeral(http).await?;
            },
            _ => interaction.defer(http).await?,
        }

        let user = match options.remove("user") {
            Some(ResolvedValue::User(user, _)) => user,
            _ => &interaction.user,
        };

        let row = Manager::rank_row(pool, user.id).await?.unwrap_or_default();

        let level = row.level();
        let xp_for_next_level = level_up_xp(level);

        let user_rank = Manager::user_rank(pool, user.id)
            .await?
            .map_or_else(|| String::from("N/A"), |rank| format!("{rank}"));

        let xp = row.xp();

        let embed = CreateEmbed::new()
            .title(format!("XP stats for {}", user.name))
            .description(format!(
                "Rank: #{user_rank}\nLevel: {level}\nXP: {xp}/{xp_for_next_level} ({}%)",
                (f64::from(xp) / f64::from(xp_for_next_level) * 100.0).round()
            ));

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("rank")
            .description("Get your rank or another member's rank")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to get the xp of",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "ephemeral",
                "Whether the response should be ephemeral",
            ))
    }
}
