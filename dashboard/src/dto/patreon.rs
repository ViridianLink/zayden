pub const OUTCOME_PARAM: &str = "patreon";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatreonOutcome {
    Connected,
    Disconnected,
    Declined,
    NoCampaign,
    Forbidden,
    StateMismatch,
    Unconfigured,
    Error,
}

impl PatreonOutcome {
    pub const ALL: [Self; 8] = [
        Self::Connected,
        Self::Disconnected,
        Self::Declined,
        Self::NoCampaign,
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
            Self::NoCampaign => "no_campaign",
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
            Self::Declined | Self::NoCampaign => "warning",
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
            | Self::NoCampaign => "status",
            Self::Forbidden
            | Self::StateMismatch
            | Self::Unconfigured
            | Self::Error => "alert",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::Connected => {
                "Patreon connected. Choose where its posts should be announced."
            },
            Self::Disconnected => {
                "Patreon disconnected. Zayden will stop announcing this campaign."
            },
            Self::Declined => "Authorisation was cancelled, so nothing changed.",
            Self::NoCampaign => {
                "That Patreon account has no campaign Zayden can read. Authorise \
                 with the creator's own account."
            },
            Self::Forbidden => {
                "Only an administrator of this server can change its Patreon \
                 connection."
            },
            Self::StateMismatch => {
                "That authorisation link expired or came back to a different \
                 browser. Start the connection again."
            },
            Self::Unconfigured => {
                "Patreon is not configured on this instance, so there is nothing \
                 to connect to."
            },
            Self::Error => {
                "Something went wrong talking to Patreon. Nothing was changed - \
                 try again."
            },
        }
    }
}
