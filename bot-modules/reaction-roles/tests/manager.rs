use reaction_roles::ReactionRole;
use serenity::all::{GenericChannelId, GuildId, MessageId, RoleId};

/// The `reaction_roles` row stores Discord snowflakes in signed `BIGINT`
/// columns; the `ReactionRole` accessors reinterpret each `i64` back to the
/// original `u64` snowflake. This pins that mapping across the CC-1
/// concrete-`PgPool` migration: each accessor must read *its own* column
/// (a field-swap regression would surface here) and preserve the value.
#[test]
fn snowflake_accessors_preserve_their_column() {
    let row = ReactionRole {
        id: 7,
        guild_id: 1_100_000_000_000_000_001,
        channel_id: 1_200_000_000_000_000_002,
        message_id: 1_300_000_000_000_000_003,
        role_id: 1_400_000_000_000_000_004,
        emoji: "✅".to_string(),
    };

    assert_eq!(row.guild_id(), GuildId::new(1_100_000_000_000_000_001));
    assert_eq!(row.channel_id(), GenericChannelId::new(1_200_000_000_000_000_002));
    assert_eq!(row.message_id(), MessageId::new(1_300_000_000_000_000_003));
    assert_eq!(row.role_id(), RoleId::new(1_400_000_000_000_000_004));
}
