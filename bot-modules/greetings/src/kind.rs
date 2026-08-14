use std::fmt::Display;

use crate::error::GreetingsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GreetingKind {
    Morning,
    Night,
}

impl GreetingKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Night => "night",
        }
    }

    #[must_use]
    pub const fn subcommand_name(self) -> &'static str {
        self.as_str()
    }

    #[must_use]
    pub const fn subcommand_description(self) -> &'static str {
        match self {
            Self::Morning => "Wish someone a good morning",
            Self::Night => "Wish someone a good night",
        }
    }

    #[must_use]
    pub const fn image_alt(self) -> &'static str {
        match self {
            Self::Morning => "Good morning image",
            Self::Night => "Good night image",
        }
    }

    #[must_use]
    pub const fn default_message(self) -> &'static str {
        match self {
            Self::Morning => "Good morning, {user}!",
            Self::Night => "Good night, {user}!",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, GreetingsError> {
        match raw.trim() {
            "morning" => Ok(Self::Morning),
            "night" => Ok(Self::Night),
            other => Err(GreetingsError::UnknownKind(other.to_string())),
        }
    }
}

impl Display for GreetingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
