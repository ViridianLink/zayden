use crate::guild::dto::Tier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub level: i32,
    pub xp: i32,
    pub message_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardPage {
    pub entries: Vec<LeaderboardEntry>,
    pub has_next: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionRoleInfo {
    pub channel_id: String,
    pub message_id: String,
    pub role_id: String,
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreetingImageInfo {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CooldownView {
    pub user_secs: i32,
    pub guild_secs: i32,
    pub floor_user_secs: i32,
    pub floor_guild_secs: i32,
    pub tier: Tier,
    pub next_tier: Option<Tier>,
    pub next_floor_user_secs: i32,
    pub next_floor_guild_secs: i32,
}

impl CooldownView {
    #[must_use]
    pub fn user_label(&self) -> String {
        format!("Per-member cooldown (seconds, min {})", self.floor_user_secs)
    }

    #[must_use]
    pub fn guild_label(&self) -> String {
        format!("Server-wide cooldown (seconds, min {})", self.floor_guild_secs)
    }

    /// The sentence pitching the next plan's lower floors, if one is offered.
    #[must_use]
    pub fn upgrade_pitch(&self) -> Option<String> {
        self.next_tier.map(|next| {
            format!(
                "On {} these floors drop to {}s and {}s.",
                next.label(),
                self.next_floor_user_secs,
                self.next_floor_guild_secs,
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreetingsView {
    pub morning_message: String,
    pub night_message: String,
    pub morning: Vec<GreetingImageInfo>,
    pub night: Vec<GreetingImageInfo>,
    pub allowed_channels: Option<Vec<String>>,
    pub channels_locked: bool,
    pub cooldowns: CooldownView,
}
