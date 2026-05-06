import { IMAGE_URL, type Snowflake } from "./common";

export interface User {
    /**
     * The ID of the user
     */
    id: Snowflake;
    /**
     * The user's username, may be unique across the platform (2-32 characters)
     */
    username: string;
    /**
     * The user's stringified 4-digit Discord tag
     */
    discriminator: string;
    /**
     * The user's display name (1-32 characters)
     */
    global_name: string | null;
    /**
     * The user's avatar hash
     */
    avatar: string | null;
    /**
     * The user's avatar decoration
     */
    // avatar_decoration_data: AvatarDecorationData | null;
    /**
     * The user's equipped collectibles
     */
    // collectibles?: Collectibles | null;
    /**
     * The user's display name style
     */
    // display_name_styles?: DisplayNameStyle | null;
    /**
     * The primary guild of the user
     */
    // primary_guild?: PrimaryGuild | null;
    /**
     * The linked users connected to the account via Family Center
     */
    // linked_users: LinkedUser[];
    /**
     * Whether the user is a bot account
     */
    bot?: boolean;
    /**
     * Whether the user is an official Discord System user (part of the urgent message system)
     */
    system?: boolean;
    /**
     * Whether the user has multi-factor authentication enabled on their account
     */
    mfa_enabled: boolean;
    /**
     * Whether the user is allowed to see NSFW content, null if not yet known
     */
    nsfw_allowed?: boolean | null;
    /**
     * The age verification status of the user
     */
    age_verification_status: number;
    /**
     * The user's pronouns (max 40 characters)
     */
    pronouns?: string;
    /**
     * The user's bio (max 190 characters)
     */
    bio: string;
    /**
     * The user's banner hash
     */
    banner: string | null;
    /**
     * The user's banner color encoded as an integer representation of a hexadecimal color code
     */
    accent_color: number | null;
    /**
     * The language option chosen by the user
     */
    locale?: string;
    /**
     * Whether the email on this account has been verified
     */
    verified: boolean;
    /**
     * The user's email address
     */
    email: string | null;
    /**
     * The user's E.164-formatted phone number
     */
    phone?: string | null;
    /**
     * Whether the user is subscribed to Nitro
     * @deprecated
     */
    premium: boolean;
    /**
     * The type of premium (Nitro) subscription on a user's account
     */
    premium_type: number;
    /**
     * The ID of the user's personal, non-employee user account
     */
    personal_connection_id?: Snowflake;
    /**
     * The flags on a user's account
     */
    flags: number;
    /**
     * The public flags on a user's account
     */
    public_flags: number;
    /**
     * The purchased flags on a user's account
     */
    purchased_flags?: number;
    /**
     * The premium usage flags on a user's account
     */
    premium_usage_flags?: number;
    /**
     * Whether the user has used the desktop client before
     */
    desktop?: boolean;
    /**
     * Whether the user has used the mobile client before
     */
    mobile?: boolean;
    /**
     * Whether the user's email has failed to deliver and is no longer valid
     */
    has_bounced_email?: boolean;
    /**
     * The types of multi-factor authenticators the user has enabled
     */
    authenticator_types?: number;
}

export function displayName(user: User): string {
    return user.global_name ?? user.username;
}

export function avatar(user: User): string {
    return `${IMAGE_URL}/avatars/${user.id}/${user.avatar}.webp`;
}

export interface PartialUser {
    /**
     * The ID of the user
     */
    id: Snowflake;
    /**
     * The user's username, may be unique across the platform (2-32 characters)
     */
    username: string;
    /**
     * The user's stringified 4-digit Discord tag
     */
    discriminator: string;
    /**
     * The user's display name (1-32 characters)
     */
    global_name?: string | null;
    /**
     * The user's avatar hash
     */
    avatar: string | null;
    /**
     * The user's avatar decoration
     */
    // avatar_decoration_data?: AvatarDecorationData | null;
    /**
     * The user's equipped collectibles
     */
    // collectibles?: Collectibles | null;
    /**
     * The user's display name style
     */
    // display_name_styles?: DisplayNameStyle | null;
    /**
     * The primary guild of the user
     */
    // primary_guild?: PrimaryGuild | null;
    /**
     * Whether the user is a bot account
     */
    bot?: boolean;
    /**
     * Whether the user is an official Discord System user (part of the urgent message system)
     */
    system?: boolean;
    /**
     * The user's banner hash
     */
    banner?: string | null;
    /**
     * The user's banner color encoded as an integer representation of a hexadecimal color code
     */
    accent_color?: number | null;
    /**
     * The public flags on a user's account
     */
    public_flags?: number;
}
