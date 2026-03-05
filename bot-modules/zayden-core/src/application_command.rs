use async_trait::async_trait;
use serde_json::Value;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{Database, Pool};

#[async_trait]
pub trait ApplicationCommand<E: std::error::Error, Db: Database>: Send + Sync {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<(), E>;

    fn command(&self) -> CreateCommand<'_>;

    fn name(&self) -> String {
        let command = self.command();

        if let Ok(Value::Object(mut map)) = serde_json::to_value(command)
            && let Some(Value::String(name)) = map.remove("name")
        {
            return name;
        }

        String::new()
    }
}

#[macro_export]
macro_rules! application_commands {
    ( $ErrorType:ty, $DatabaseType:ty; $( $cmd:ident ),* $(,)? ) => {
        {
            let mut commands: ::std::collections::HashMap<
                String,
                Box<dyn ApplicationCommand<$ErrorType, $DatabaseType>>,
            > = ::std::collections::HashMap::new();

            $(
                let cmd_instance = $cmd;
                commands.insert(
                    cmd_instance.name(),
                    Box::new(cmd_instance)
                );
            )*
            commands
        }
    };
}
