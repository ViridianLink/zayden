use std::fmt::Write as _;

use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateAllowedMentions,
    CreateButton,
    CreateComponent,
    CreateMessage,
    Mentionable,
    RoleId,
    UserId,
};

use crate::idle::Ball;

pub const SOLVED_ID: &str = "support_solved";
pub const STILL_OPEN_ID: &str = "support_still_open";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nudge {
    Op,
    Helper,
    Unanswered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub kind: Nudge,
    pub users: Vec<UserId>,
    pub roles: Vec<RoleId>,
}

#[must_use]
pub fn reminder(
    ball: Ball,
    op: UserId,
    helper: Option<UserId>,
    support_roles: &[RoleId],
) -> Option<Reminder> {
    match (ball, helper) {
        (Ball::Op, _) => {
            Some(Reminder { kind: Nudge::Op, users: vec![op], roles: Vec::new() })
        },
        (Ball::Helper, Some(helper)) => Some(Reminder {
            kind: Nudge::Helper,
            users: vec![helper],
            roles: Vec::new(),
        }),
        (Ball::Helper, None) if !support_roles.is_empty() => Some(Reminder {
            kind: Nudge::Unanswered,
            users: Vec::new(),
            roles: support_roles.to_vec(),
        }),
        (Ball::Helper, None) => None,
    }
}

impl Reminder {
    #[must_use]
    pub fn mentions(&self) -> String {
        let mut line = String::new();

        for role in &self.roles {
            let _ = write!(line, "{} ", role.mention());
        }

        for user in &self.users {
            let _ = write!(line, "{} ", user.mention());
        }

        line.trim_end().to_owned()
    }

    #[must_use]
    pub fn text(&self, since: i64) -> String {
        let body = match self.kind {
            Nudge::Op => format!(
                "Just checking in - the last reply here was <t:{since}:R> and \
                 we have not heard back. Did that sort it out?"
            ),
            Nudge::Helper => format!(
                "This one has been waiting on you since <t:{since}:R>. Could \
                 you take another look when you get a chance?"
            ),
            Nudge::Unanswered => format!(
                "This ticket has been waiting for a first reply since \
                 <t:{since}:R>."
            ),
        };

        format!("{}\n{body}", self.mentions())
    }

    pub fn allowed_mentions(&self) -> CreateAllowedMentions<'_> {
        CreateAllowedMentions::new()
            .everyone(false)
            .all_users(false)
            .all_roles(false)
            .users(self.users.as_slice())
            .roles(self.roles.as_slice())
    }

    #[must_use]
    pub fn components<'a>(&self) -> Vec<CreateComponent<'a>> {
        if self.kind != Nudge::Op {
            return Vec::new();
        }

        vec![CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            CreateButton::new(SOLVED_ID).label("Solved").style(ButtonStyle::Success),
            CreateButton::new(STILL_OPEN_ID)
                .label("Still need help")
                .style(ButtonStyle::Secondary),
        ]))]
    }

    pub fn message(&self, since: i64) -> CreateMessage<'_> {
        CreateMessage::new()
            .content(self.text(since))
            .allowed_mentions(self.allowed_mentions())
            .components(self.components())
    }
}
