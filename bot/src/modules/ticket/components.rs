use serenity::all::{
    ComponentInteraction, CreateInputText, CreateLabel, CreateModalComponent, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, Http, InputTextStyle,
};
use ticket::TicketComponent;

use crate::Result;

use super::Ticket;

impl Ticket {
    pub async fn ticket_create(http: &Http, component: &ComponentInteraction) -> Result<()> {
        let components = vec![
            CreateModalComponent::Label(CreateLabel::select_menu(
                "Select a relevent category:",
                CreateSelectMenu::new(
                    "ticket_category",
                    CreateSelectMenuKind::String {
                        options: vec![
                            CreateSelectMenuOption::new("Complaint", "complaint"),
                            CreateSelectMenuOption::new("General question", "general_question"),
                            CreateSelectMenuOption::new("Role Application", "role_application"),
                        ]
                        .into(),
                    },
                ),
            )),
            CreateModalComponent::Label(CreateLabel::input_text(
                "Describe the issue:",
                CreateInputText::new(InputTextStyle::Paragraph, "ticket_body"),
            )),
        ];

        TicketComponent::ticket_create(http, component, components).await?;

        Ok(())
    }
}
