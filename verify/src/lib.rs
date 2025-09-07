use serenity::all::{
    ButtonStyle, Colour, CommandInteraction, ComponentInteraction, CreateButton, CreateCommand,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, Http,
    Permissions, RoleId,
};

const VERIFIED_ROLE: RoleId = RoleId::new(1404640603848839299);

pub struct Panel;

impl Panel {
    pub async fn run_command(http: &Http, interaction: &CommandInteraction) {
        let embed = CreateEmbed::new()
            .description("Click the green button below to verify")
            .colour(Colour::DARK_GREEN);

        let button = CreateButton::new("verify")
            .label("Verify")
            .style(ButtonStyle::Success);

        interaction
            .channel_id
            .send_message(http, CreateMessage::new().embed(embed).button(button))
            .await
            .unwrap();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Panel sent!")
                        .ephemeral(true),
                ),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("panel")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .description("Send a verification panel/button in this channel")
    }

    pub async fn run_component(
        http: &Http,
        interaction: &ComponentInteraction,
    ) -> Result<(), serenity::Error> {
        let member = interaction.member.as_ref().unwrap();

        member
            .add_role(http, VERIFIED_ROLE, Some("Verified user"))
            .await
            .unwrap();

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
