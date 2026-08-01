pub mod error;
pub use error::{Result, VerifyError};
use serenity::all::{
    ButtonStyle,
    Colour,
    CommandInteraction,
    ComponentInteraction,
    CreateButton,
    CreateCommand,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    GuildId,
    Http,
    Permissions,
    RoleId,
};
use zayden_app::config::{RolesSettingsRow, SettingsStore};
use zayden_core::{as_i64, as_u64};

pub async fn verified_role(
    store: &SettingsStore<RolesSettingsRow>,
    guild_id: GuildId,
) -> Result<RoleId> {
    store
        .get(as_i64(guild_id.get()))
        .await?
        .verified_role_id
        .map(|id| RoleId::new(as_u64(id)))
        .ok_or(VerifyError::RoleNotConfigured)
}

pub struct Panel;

impl Panel {
    pub async fn run_command(
        http: &Http,
        interaction: &CommandInteraction,
    ) -> Result<()> {
        let embed = CreateEmbed::new()
            .description("Click the green button below to verify")
            .colour(Colour::DARK_GREEN);

        let button =
            CreateButton::new("verify").label("Verify").style(ButtonStyle::Success);

        interaction
            .channel_id
            .send_message(http, CreateMessage::new().embed(embed).button(button))
            .await?;

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Panel sent!")
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("panel")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .description("Send a verification panel/button in this channel")
    }

    pub async fn run_component(
        http: &Http,
        interaction: &ComponentInteraction,
        store: &SettingsStore<RolesSettingsRow>,
    ) -> Result<()> {
        let Some(member) = interaction.member.as_ref() else {
            return Err(VerifyError::NotGuildMember);
        };

        let role = verified_role(store, member.guild_id).await?;

        member.add_role(http, role, Some("Verified user")).await?;

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("You have been verified.")
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }
}
