import type { Snowflake } from "./common";
import type { PartialUser } from "./user";

export interface GuildMember {
    /**
     * The user this guild member represents
     */
    user: PartialUser;
    /**
     * The guild-specific nickname of the member (1-32 characters)
     */
    nick?: string | null;
    /**
     * The member's guild avatar hash
     */
    avatar?: string | null;
    /**
     * The member's guild avatar decoration
     */
    // avatar_decoration_data?: AvatarDecorationData | null;
    /**
     * The member's equipped collectibles
     */
    // collectibles?: Collectibles | null;
    /**
     * The member's guild banner hash
     */
    banner?: string | null;
    /**
     * The member's guild-specific bio (max 190 characters)
     */
    bio?: string | null;
    /**
     * The role IDs assigned to this member
     */
    roles: Snowflake[];
    /**
     * When the user joined the guild
     */
    joined_at: string;
    /**
     * When the member subscribed to (started boosting ) the guild
     */
    premium_since?: string | null;
    /**
     * Whether the member is deafened in voice channels
     */
    deaf?: boolean;
    /**
     * Whether the member is muted in voice channels
     */
    mute?: boolean;
    /**
     * Whether the member has not yet passed the guild's member verification requirements
     */
    pending?: boolean;
    /**
     * When the member's timeout will expire and they will be able to communicate in the guild again
     */
    communication_disabled_until?: string | null;
    /**
     * When the member's unusual DM activity flag will expire
     */
    unusual_dm_activity_until?: string | null;
    /**
     * The member's flags
     */
    flags: number;
    /**
     * Total permissions of the member in the guild
     */
    permissions?: string;
}

export function displayName(member: GuildMember): string {
    return member.nick ?? member.user.global_name ?? member.user.username;
}
