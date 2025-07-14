mod create;
mod join;
mod joined;
mod leave;
mod setup;
mod tags;
mod timezone;

pub use joined::{JoinedManager, JoinedRow};
use serenity::all::{
    AutocompleteChoice, AutocompleteOption, CommandInteraction, CommandOptionType,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    Http, ResolvedOption, ResolvedValue,
};
pub use setup::SetupManager;
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{ACTIVITIES, PostManager, PostRow, Result, Savable, TimezoneManager};

pub struct Command;

impl Command {
    pub async fn lfg<
        Db: Database,
        TzManager: TimezoneManager<Db>,
        PostHandler: PostManager<Db> + SetupManager<Db> + JoinedManager<Db> + Savable<Db, PostRow>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let command = options.pop().unwrap();

        let options = match command.value {
            ResolvedValue::SubCommand(options) => options,
            ResolvedValue::SubCommandGroup(options) => options,
            _ => unreachable!("Subcommand is required"),
        };
        let options = parse_options(options);

        match command.name {
            "setup" => Self::setup::<Db, PostHandler>(http, interaction, pool, options).await?,
            "create" => Self::create::<Db, TzManager>(http, interaction, pool, options).await?,
            "tags" => Self::tags::<Db, PostHandler>(http, interaction, pool, options).await?,
            "join" => Self::join::<Db, PostHandler>(http, interaction, pool, options).await?,
            "leave" => Self::leave::<Db, PostHandler>(http, interaction, pool).await?,
            "joined" => Self::joined::<Db, PostHandler>(http, interaction, pool).await,
            "timezone" => Self::timezone::<Db, TzManager>(http, interaction, pool, options).await?,
            _ => unreachable!("Invalid subcommand"),
        }

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let setup = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "setup",
            "Setup the lfg plugin",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "The channel to create the lfg threads in",
            )
            .required(true),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Role,
            "role",
            "The role to mention when a new lfg thread is created",
        ));

        let create = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "create",
            "Create a new looking for group post",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "activity",
                "The activity you are looking to do",
            )
            .required(true)
            .set_autocomplete(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "template",
                "The embed template for the event",
            )
            .add_string_choice("Default", "0"),
        );

        let tags = CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            "tags",
            "Edit the tags for the lfg post",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "add",
            "Add tags to the lfg post",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "remove",
            "Remove tags from the lfg post",
        ));

        let join = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "join",
            "Join a looking for group post",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "thread",
            "The LFG thread",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::User,
            "guardian",
            "The guardian you want to join",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "alternate",
            "Join as an alternate",
        ));

        let leave = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "leave",
            "Leave a looking for group post",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "thread",
            "The LFG thread",
        ));

        let joined = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "joined",
            "View all the posts you have joined",
        );

        let timezone = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "timezone",
            "Set your timezone",
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "region", "Your region")
                .required(true)
                .set_autocomplete(true),
        );

        CreateCommand::new("lfg")
            .description("Create a looking for group post")
            .add_option(setup)
            .add_option(create)
            .add_option(tags)
            .add_option(join)
            .add_option(leave)
            .add_option(joined)
            .add_option(timezone)
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
    ) -> Result<()> {
        let command = &interaction.data.options().remove(0);

        let opt_value = option.value.to_lowercase();

        let filtered = match command.name {
            "create" => ACTIVITIES
                .iter()
                .filter(|activity| activity.name.to_lowercase().contains(&opt_value))
                .take(25)
                .map(|activity| AutocompleteChoice::new(activity.name, activity.name))
                .collect::<Vec<_>>(),

            "timezone" => chrono_tz::TZ_VARIANTS
                .iter()
                .filter(|tz| tz.name().to_lowercase().contains(&opt_value))
                .take(25)
                .map(|tz| AutocompleteChoice::new(tz.name(), tz.name()))
                .collect::<Vec<_>>(),
            _ => return Ok(()),
        };

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(filtered),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }
}
