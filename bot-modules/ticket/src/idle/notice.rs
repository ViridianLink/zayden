use serenity::all::{CreateAllowedMentions, CreateMessage, Mentionable, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notice {
    pub op: UserId,
    pub since: i64,
}

impl Notice {
    #[must_use]
    pub const fn new(op: UserId, since: i64) -> Self {
        Self { op, since }
    }

    #[must_use]
    pub fn text(&self) -> String {
        format!(
            "{}\nClosing this for now - the last reply was <t:{}:R> and we \
             have not heard back since the reminder. Open a new post if it \
             comes back and we will pick it up from there.",
            self.op.mention(),
            self.since
        )
    }

    pub fn allowed_mentions<'a>(&self) -> CreateAllowedMentions<'a> {
        CreateAllowedMentions::new()
            .everyone(false)
            .all_users(false)
            .all_roles(false)
            .users(vec![self.op])
    }

    pub fn message<'a>(&self) -> CreateMessage<'a> {
        CreateMessage::new()
            .content(self.text())
            .allowed_mentions(self.allowed_mentions())
    }
}
