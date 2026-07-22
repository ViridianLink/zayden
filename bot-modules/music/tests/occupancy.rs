use music::VoiceOccupancy;
use serenity::all::{ChannelId, GuildId, UserId, VoiceState};

const BOT: UserId = UserId::new(1);
const LISTENER: UserId = UserId::new(2);

const GUILD_A: GuildId = GuildId::new(10);
const GUILD_B: GuildId = GuildId::new(20);

const CHAN_A: ChannelId = ChannelId::new(100);
const CHAN_B: ChannelId = ChannelId::new(200);

/// Build the smallest JSON gateway payload that populates the three fields
/// `VoiceOccupancy::update` reads (`guild_id`, `user_id`, `channel_id`).
///
/// `VoiceState` has no public constructor, so tests deserialize one; each test
/// turns this into a `VoiceState` inside its own body (where clippy permits the
/// fallible `expect`).
fn voice_state_json(
    guild: GuildId,
    user: UserId,
    channel: Option<ChannelId>,
) -> String {
    let channel_field =
        channel.map_or_else(|| "null".to_string(), |c| format!("\"{c}\""));

    format!(
        r#"{{
            "guild_id": "{guild}",
            "user_id": "{user}",
            "channel_id": {channel_field},
            "session_id": "sess",
            "deaf": false,
            "mute": false,
            "self_deaf": false,
            "self_mute": false,
            "self_video": false,
            "suppress": false,
            "request_to_speak_timestamp": null
        }}"#
    )
}

/// DS-1: a listener present in two guilds at once must be counted independently
/// per guild. Joining guild B must not evict their presence in guild A, and
/// leaving guild B must not remove them from guild A.
#[test]
fn presence_is_scoped_per_guild() {
    let occupancy = VoiceOccupancy::new();
    let update = |guild, user, channel| {
        let state: VoiceState =
            serde_json::from_str(&voice_state_json(guild, user, channel))
                .expect("valid VoiceState payload");
        occupancy.update(&state);
    };

    // Listener joins guild A's channel (where the bot is playing).
    update(GUILD_A, LISTENER, Some(CHAN_A));
    assert_eq!(
        occupancy.non_bot_count(GUILD_A, CHAN_A, BOT),
        1,
        "listener should be counted in guild A after joining"
    );

    // Same listener also joins guild B's voice. Before the fix this overwrote
    // the single UserId-keyed entry and dropped guild A's count to 0.
    update(GUILD_B, LISTENER, Some(CHAN_B));
    assert_eq!(
        occupancy.non_bot_count(GUILD_A, CHAN_A, BOT),
        1,
        "joining guild B must not evict the listener from guild A"
    );
    assert_eq!(
        occupancy.non_bot_count(GUILD_B, CHAN_B, BOT),
        1,
        "listener should also be counted in guild B"
    );

    // Listener leaves guild B (channel_id = None for guild B). Before the fix
    // this removed the user globally, corrupting guild A's count.
    update(GUILD_B, LISTENER, None);
    assert_eq!(
        occupancy.non_bot_count(GUILD_A, CHAN_A, BOT),
        1,
        "leaving guild B must not remove the listener from guild A"
    );
    assert_eq!(
        occupancy.non_bot_count(GUILD_B, CHAN_B, BOT),
        0,
        "listener should be gone from guild B after leaving"
    );
}

/// The per-guild lookup helpers must likewise not leak across guilds.
#[test]
fn channel_of_is_scoped_per_guild() {
    let occupancy = VoiceOccupancy::new();
    let update = |guild, user, channel| {
        let state: VoiceState =
            serde_json::from_str(&voice_state_json(guild, user, channel))
                .expect("valid VoiceState payload");
        occupancy.update(&state);
    };

    update(GUILD_A, LISTENER, Some(CHAN_A));
    update(GUILD_B, LISTENER, Some(CHAN_B));

    assert_eq!(occupancy.channel_of(GUILD_A, LISTENER), Some(CHAN_A));
    assert_eq!(occupancy.channel_of(GUILD_B, LISTENER), Some(CHAN_B));

    // Membership sets are per (guild, channel).
    assert!(occupancy.members_in_channel(GUILD_A, CHAN_A).contains(&LISTENER));
    assert!(!occupancy.members_in_channel(GUILD_A, CHAN_A).contains(&BOT));
    assert!(occupancy.members_in_channel(GUILD_B, CHAN_B).contains(&LISTENER));
    assert!(occupancy.members_in_channel(GUILD_B, CHAN_A).is_empty());
}
