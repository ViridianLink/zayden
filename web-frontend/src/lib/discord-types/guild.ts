import { IMAGE_URL, type Snowflake } from "./common";
import type { Role } from "./role";

export interface Guild {
    id: Snowflake;
    name: string;
    icon: string | null;
    banner: string | null;
    home_header: string | null;
    splash: string | null;
    discovery_splash: string | null;
    owner_id: Snowflake;
    application_id: Snowflake | null;
    description: string | null;
    region?: string | null;
    afk_channel_id: Snowflake | null;
    afk_timeout: number;
    widget_enabled?: boolean;
    widget_channel_id?: Snowflake | null;
    verification_level: number;
    default_message_notifications: number;
    explicit_content_filter: number;
    features: string[];
    roles: Role[];
    // emojis: Emoji[];
    // stickers: Sticker[];
    mfa_level: number;
    system_channel_id: Snowflake | null;
    system_channel_flags: number;
    rules_channel_id: Snowflake | null;
    public_updates_channel_id: Snowflake | null;
    safety_alerts_channel_id: Snowflake | null;
    max_presences?: number | null;
    max_members?: number;
    vanity_url_code: string | null;
    premium_tier: number;
    premium_subscription_count: number;
    preferred_locale: string;
    max_video_channel_users?: number;
    max_stage_video_channel_users?: number;
    nsfw: boolean;
    nsfw_level: number;
    owner_configured_content_level: number | null;
    hub_type: number | null;
    premium_progress_bar_enabled: boolean;
    latest_onboarding_question_id: Snowflake | null;
    // incidents_data: AutomodIncidentsData | null;
    // inventory_settings: GuildInventorySettings | null;
    // premium_features: GuildPremiumFeatures | null;
    // profile: GuildIdentity | null;
    approximate_member_count?: number;
    approximate_presence_count?: number;
}

export function icon(guild: Guild): string {
    if (guild.icon) {
        return `${IMAGE_URL}/icons/${guild.id}/${guild.icon}.webp`;
    }

    return "https://cdn.discordapp.com/app-icons/787490197943091211/e0c04aa4831f1f329669fe83f79a18ac.webp";
}
