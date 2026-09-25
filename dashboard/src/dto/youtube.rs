pub const OUTCOME_PARAM: &str = "youtube";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum YoutubeOutcome {
    Connected,
    Disconnected,
    Declined,
    NoChannel,
    Forbidden,
    StateMismatch,
    Unconfigured,
    Error,
}

impl YoutubeOutcome {
    pub const ALL: [Self; 8] = [
        Self::Connected,
        Self::Disconnected,
        Self::Declined,
        Self::NoChannel,
        Self::Forbidden,
        Self::StateMismatch,
        Self::Unconfigured,
        Self::Error,
    ];

    pub const fn as_key(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Declined => "declined",
            Self::NoChannel => "no_channel",
            Self::Forbidden => "forbidden",
            Self::StateMismatch => "state_mismatch",
            Self::Unconfigured => "unconfigured",
            Self::Error => "error",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|outcome| outcome.as_key() == key)
    }

    pub const fn class(self) -> &'static str {
        match self {
            Self::Connected | Self::Disconnected => "success",
            Self::Declined | Self::NoChannel => "warning",
            Self::Forbidden
            | Self::StateMismatch
            | Self::Unconfigured
            | Self::Error => "error",
        }
    }

    pub const fn role(self) -> &'static str {
        match self {
            Self::Connected
            | Self::Disconnected
            | Self::Declined
            | Self::NoChannel => "status",
            Self::Forbidden
            | Self::StateMismatch
            | Self::Unconfigured
            | Self::Error => "alert",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::Connected => {
                "YouTube connected. Choose where its uploads should be announced."
            },
            Self::Disconnected => {
                "YouTube disconnected. Zayden will stop announcing this channel."
            },
            Self::Declined => "Authorisation was cancelled, so nothing changed.",
            Self::NoChannel => {
                "That Google account has no YouTube channel. Sign in with the \
                 account (or brand account) that owns the channel."
            },
            Self::Forbidden => {
                "Only an administrator of this server can change its YouTube \
                 connection."
            },
            Self::StateMismatch => {
                "That YouTube authorisation link expired or came back to a \
                 different browser. Start the connection again."
            },
            Self::Unconfigured => {
                "YouTube is not configured on this instance, so there is nothing \
                 to connect to."
            },
            Self::Error => {
                "Something went wrong talking to YouTube. Nothing was changed - \
                 try again."
            },
        }
    }
}
