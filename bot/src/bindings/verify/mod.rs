use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    Permissions,
    RoleId,
    User,
};
use zayden_core::{
    ComponentCtx,
    CoreError,
    HandlerError,
    IdMatch,
    InvocationCtx,
    ModuleCommand,
    ModuleComponent,
    SubCommandOptions,
    sole_option,
};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub struct Panel;

#[async_trait]
impl ModuleCommand for Panel {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("panel")
    }

    fn definition(&self) -> CreateCommand<'static> {
        verify::Panel::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        verify::Panel::run_command(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Panel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("verify"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        verify::Panel::run_component(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

pub struct ManVerify;

#[async_trait]
impl ModuleCommand for ManVerify {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("manverify")
    }

    fn definition(&self) -> CreateCommand<'static> {
        CreateCommand::new("manverify")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .description("Manually verifies a user")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "User to verify",
                )
                .required(true),
            )
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        const VERIFIED_ROLE: RoleId = RoleId::new(1_404_640_603_848_839_299);

        let user: &User = {
            let mut options = cx.interaction.data.options();
            let options: SubCommandOptions<'_> = sole_option(&mut options)?;
            sole_option(&mut options.into_vec())?
        };

        let guild_id = cx.interaction.guild_id.ok_or(CoreError::MissingGuildId)?;

        cx.ctx
            .http
            .add_member_role(
                guild_id,
                user.id,
                VERIFIED_ROLE,
                Some(&format!(
                    "User manually verified by {}.",
                    cx.interaction.user.display_name()
                )),
            )
            .await?;

        Ok(())
    }
}

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder.add_command(Panel).add_command(ManVerify).add_component(Panel)?;
    Ok(())
}
