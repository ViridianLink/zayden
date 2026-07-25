use serenity::all::ReactionType;

use crate::{ReactionRoleError, Result};

pub struct ParsedEmoji {
    pub stored: String,
    pub custom_id: Option<u64>,
    pub name: String,
}

impl ParsedEmoji {
    pub fn parse(input: &str) -> Result<Self> {
        let reaction = ReactionType::try_from(input.trim())?;
        let stored = reaction.to_string();

        match reaction {
            ReactionType::Custom { id, name, .. } => Ok(Self {
                stored,
                custom_id: Some(id.get()),
                name: name.map(|n| n.to_string()).unwrap_or_default(),
            }),
            ReactionType::Unicode(unicode) => {
                Ok(Self { stored, custom_id: None, name: unicode.to_string() })
            },
            _ => Err(ReactionRoleError::UnsupportedEmoji(stored)),
        }
    }
}
