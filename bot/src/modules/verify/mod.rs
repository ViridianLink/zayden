use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, ComponentInteraction, Context, CreateCommand,
    CreateCommandOption, Permissions, ResolvedOption, ResolvedValue, RoleId,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Component};

use crate::{Error, Result};

pub struct Panel;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Panel {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        verify::Panel::run_command(&ctx.http, interaction).await;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(verify::Panel::register())
    }
}

#[async_trait]
impl Component<Error, Postgres> for Panel {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, _pool: &PgPool) -> Result<()> {
        verify::Panel::run_component(&ctx.http, interaction).await?;
        Ok(())
    }
}

pub struct ManVerify;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for ManVerify {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        const VERIFIED_ROLE: RoleId = RoleId::new(1404640603848839299);

        let Some(ResolvedValue::User(user, _)) = options.pop().map(|opt| opt.value) else {
            unreachable!("User option is required")
        };

        let guild_id = interaction.guild_id.unwrap();

        ctx.http
            .add_member_role(
                guild_id,
                user.id,
                VERIFIED_ROLE,
                Some(&format!(
                    "User manually verified by {}.",
                    interaction.user.display_name()
                )),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        let cmd = CreateCommand::new("manverify")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .description("Manually verifies a user")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "User to verify")
                    .required(true),
            );

        Ok(cmd)
    }
}
