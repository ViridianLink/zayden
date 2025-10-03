import { navigate } from "svelte-routing";
import Cookies from "js-cookie";

export const InviteUrl =
    "https://discord.com/oauth2/authorize?client_id=787490197943091211&permissions=8&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fauth%2Fcallback&integration_type=0&scope=identify+bot+guilds+applications.commands";
export const BaseUrl = "https://discord.com/api/v10";
export const ImageUrl = "https://cdn.discordapp.com";

export async function get<T>(endpoint: string): Promise<T> {
    const authToken = Cookies.get("auth-token");

    if (!authToken) {
        authFail();
    }

    const response = await fetch(`${BaseUrl}${endpoint}`, {
        headers: { authorization: `Bearer ${authToken}` },
    });

    if (response.status == 401) {
        authFail();
    }

    return await response.json();
}

function authFail(): never {
    console.log("No auth token, redirecting to login");
    navigate("/login");
    throw new Error("authFail");
}

export type Snowflake = string;

export interface User {
    id: Snowflake;
    username: string;
    global_name: string | null;
    avatar: string | null;
}

export function display_name(user: User): string {
    return user.global_name ?? user.username;
}

export function avatar(user: User): string {
    return `${ImageUrl}/avatars/${user.id}/${user.avatar}.webp`;
}

export interface Guild {
    /**
     * The ID of the guild
     */
    id: Snowflake;
    /**
     * The name of the guild (2-100 characters)
     */
    name: string;
    /**
     * The guild's icon hash
     */
    icon: string | null;
    /**
     * The guild's banner hash
     */
    banner: string | null;
    /**
     * The guild's home header hash , used in new member welcome
     */
    home_header: string | null;
    /**
     * The guild's splash hash
     */
    splash: string | null;
    /**
     * The guild's discovery splash hash
     */
    discovery_splash: string | null;
    /**
     * The user ID of the guild's owner
     */
    owner_id: Snowflake;
    /**
     * The application ID of the bot that created the guild
     * @deprecated
     */
    application_id: Snowflake | null;
    /**
     * The description for the guild (max 300 characters)
     */
    description: string | null;
    /**
     * The main voice region ID of the guild
     * @deprecated
     */
    region?: string | null;
    /**
     * The ID of the guild's AFK channel; this is where members in voice idle for longer than afk_timeout are moved
     */
    afk_channel_id: Snowflake | null;
    /**
     * The AFK timeout of the guild (one of 60, 300, 900, 1800, 3600, in seconds)
     */
    afk_timeout: number;
    /**
     * Whether the guild widget is enabled
     */
    widget_enabled?: boolean;
    /**
     * The channel ID that the widget will generate an invite to, if any
     */
    widget_channel_id?: Snowflake | null;
    /**
     * The verification level required for the guild
     */
    verification_level: number;
    /**
     * Default message notification level for the guild
     */
    default_message_notifications: number;
    /**
     * Whose messages are scanned and deleted for explicit content in the guild
     */
    explicit_content_filter: number;
    /**
     * Enabled guild features
     */
    features: string[];
    /**
     * Roles in the guild
     */
    // roles: Role[];
    /**
     * Custom guild emojis
     */
    // emojis: Emoji[];
    /**
     * Custom guild stickers
     */
    // stickers: Sticker[];
    /**
     * Required MFA level for administrative actions within the guild
     */
    mfa_level: number;
    /**
     * The ID of the channel where system event messages, such as member joins and premium subscriptions (boosts), are posted
     */
    system_channel_id: Snowflake | null;
    /**
     * The flags that limit system event messages
     */
    system_channel_flags: number;
    /**
     * The ID of the channel where community guilds display rules and/or guidelines
     */
    rules_channel_id: Snowflake | null;
    /**
     * The ID of the channel where admins and moderators of community guilds receive notices from Discord
     */
    public_updates_channel_id: Snowflake | null;
    /**
     * The ID of the channel where admins and moderators of community guilds receive safety alerts from Discord
     */
    safety_alerts_channel_id: Snowflake | null;
    /**
     * The maximum number of presences for the guild ( null is usually returned, apart from the largest of guilds)
     */
    max_presences?: number | null;
    /**
     * The maximum number of members for the guild
     */
    max_members?: number;
    /**
     * The guild's vanity invite code
     */
    vanity_url_code: string | null;
    /**
     * The guild's premium tier (boost level)
     */
    premium_tier: number;
    /**
     * The number of premium subscriptions (boosts) the guild currently has
     */
    premium_subscription_count: number;
    /**
     * The preferred locale of the guild; used in discovery and notices from Discord (default "en-US")
     */
    preferred_locale: string;
    /**
     * The maximum number of users in a voice channel while someone has video enabled
     */
    max_video_channel_users?: number;
    /**
     * The maximum number of users in a stage channel while someone has video enabled
     */
    max_stage_video_channel_users?: number;
    /**
     * Whether the guild is considered NSFW ( EXPLICIT or AGE_RESTRICTED )
     * @deprecated
     */
    nsfw: boolean;
    /**
     * The NSFW level of the guild
     */
    nsfw_level: number;
    /**
     * The owner-configured NSFW level of the guild
     */
    owner_configured_content_level: number | null;
    /**
     * The type of student hub the guild is, if it is a student hub
     */
    hub_type: number | null;
    /**
     * Whether the guild has the premium (boost) progress bar enabled
     */
    premium_progress_bar_enabled: boolean;
    /**
     * The ID of the guild's latest onboarding prompt option
     */
    latest_onboarding_question_id: Snowflake | null;
    /**
     * Information on the guild's AutoMod incidents
     */
    // incidents_data: AutomodIncidentsData | null;
    /**
     * Settings for emoji packs
     * @deprecated
     */
    // inventory_settings: GuildInventorySettings | null;
    /**
     * The guild's powerup information
     */
    // premium_features: GuildPremiumFeatures | null;
    /**
     * The guild's identity
     */
    // profile: GuildIdentity | null;
    /**
     * Approximate count of total members in the guild
     */
    approximate_member_count?: number;
    /**
     * Approximate count of non-offline members in the guild
     */
    approximate_presence_count?: number;
}

export function icon(guild: Guild): string {
    if (guild.icon) {
        return `${ImageUrl}/icons/${guild.id}/${guild.icon}.webp`;
    }

    return "https://cdn.discordapp.com/app-icons/787490197943091211/e0c04aa4831f1f329669fe83f79a18ac.webp";
}
