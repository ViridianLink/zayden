use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
};

pub struct Tree;

impl Tree {
    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("tree")
            .description("Display your family tree.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user whose family tree to display.",
            ))
    }
}
