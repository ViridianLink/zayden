import type { Snowflake } from "./common";

export interface Channel {
    /**
     * The ID of the channel
     */
    id: Snowflake;
    /**
     * The type of channel
     */
    type: number;
    /**
     * The ID of the guild the channel is in
     */
    guild_id?: Snowflake;
    /**
     * Sorting position of the channel
     */
    position?: number;
    /**
     * Explicit permission overwrites for members and roles
     */
    // permission_overwrites?: PermissionOverwrite[];
    /**
     * The name of the channel (1-100 characters)
     */
    name?: string | null;
    /**
     * The channel topic (max 4096 characters)
     */
    topic?: string | null;
    /**
     * Whether the channel is NSFW
     */
    nsfw?: boolean;
    /**
     * The ID of the last message sent (or thread created for thread-only channels, guild added for directory channels) in this channel (may not point to an existing resource)
     */
    last_message_id?: Snowflake | null;
    /**
     * The bitrate (in bits) of the voice channel
     */
    bitrate?: number;
    /**
     * The user limit of the voice channel (max 99, 0 refers to no limit)
     */
    user_limit?: number;
    /**
     * Duration in seconds seconds a user has to wait before sending another message (max 21600); bots, as well as users with the permission MANAGE_MESSAGES or MANAGE_CHANNELS , are unaffected
     */
    rate_limit_per_user?: number;
    /**
     * The recipients of the private channel, excluding the requesting user
     */
    // recipients?: PartialUser[];
    /**
     * The recipient flags of the DM
     */
    recipient_flags?: number;
    /**
     * The group DM's icon hash
     */
    icon?: string | null;
    /**
     * The nicknames of the users in the group DM
     */
    // nicks?: ChannelNick[];
    /**
     * Whether the group DM is managed by an application
     */
    managed?: boolean;
    /**
     * Whether the user has acknowledged the presence of blocked users in the group DM
     */
    blocked_user_warning_dismissed?: boolean;
    /**
     * The safety warnings for the DM channel
     */
    // safety_warnings?: SafetyWarning[];
    /**
     * The ID of the application that manages the group DM
     */
    application_id?: Snowflake;
    /**
     * The ID of the owner of the group DM or thread
     */
    owner_id?: Snowflake;
    /**
     * The owner of this thread; only included on certain API endpoints
     */
    // owner?: GuildMember | null;
    /**
     * The ID of the parent category/channel for the guild channel/thread
     */
    parent_id?: Snowflake | null;
    /**
     * When the last pinned message was pinned, if any
     */
    last_pin_timestamp?: string | null;
    /**
     * The voice region ID for the voice channel (automatic when null )
     */
    rtc_region?: string | null;
    /**
     * The camera video quality mode of the voice channel (default AUTO )
     */
    video_quality_mode?: number;
    /**
     * The number of messages ever sent in a thread; similar to message_count on message creation, but will not decrement the number when a message is deleted
     */
    total_message_sent?: number;
    /**
     * The number of messages (not including the initial message or deleted messages) in a thread (if the thread was created before July 1, 2022, it stops counting at 50)
     */
    message_count?: number;
    /**
     * An approximate count of users in a thread, stops counting at 50
     */
    member_count?: number;
    /**
     * The IDs of some of the members in a thread
     */
    // member_ids_preview?: snowflake[];
    /**
     * Thread-specific channel metadata
     */
    // thread_metadata?: ThreadMetadata;
    /**
     * Thread member object for the current user, if they have joined the thread; only included on certain API endpoints
     */
    // member?: ThreadMember;
    /**
     * Default duration in minutes for newly created threads to stop showing in the channel list after inactivity (one of 60, 1440, 4320, 10080)
     */
    default_auto_archive_duration?: number | null;
    /**
     * Default duration in seconds a user has to wait before sending another message in newly created threads; this field is copied to the thread at creation time and does not live update
     */
    default_thread_rate_limit_per_user?: number;
    /**
     * Computed permissions for the invoking user in the channel, including overwrites
     */
    permissions?: string;
    /**
     * The channel's flags
     */
    flags?: number;
    /**
     * The tags that can be used in a thread-only channel (max 20)
     */
    // available_tags?: Tag[];
    /**
     * The IDs of tags that are applied to a thread in a thread-only channel (max 5)
     */
    // applied_tags?: snowflake[];
    /**
     * The emoji to show in the add reaction button on a thread in a thread-only channel
     */
    // default_reaction_emoji?: DefaultReaction | null;
    /**
     * The default layout of a thread-only channel
     */
    default_forum_layout?: number;
    /**
     * The default sort order of a thread-only channel's threads (default LATEST_ACTIVITY )
     */
    default_sort_order?: number | null;
    /**
     * The default tag search setting for a thread-only channel
     */
    default_tag_setting?: string;
    /**
     * The emoji to show next to the channel name in channels list
     */
    // icon_emoji?: IconEmoji | null;
    /**
     * Whether the DM is a message request
     */
    is_message_request?: boolean;
    /**
     * When the message request was created
     */
    is_message_request_timestamp?: string | null;
    /**
     * Whether the DM is a spam message request
     */
    is_spam?: boolean;
    /**
     * The background color of the channel icon emoji encoded as an integer representation of a hexadecimal color code
     */
    theme_color?: number | null;
    /**
     * The status of the voice channel (max 500 characters)
     */
    status?: string | null;
    /**
     * When the HD streaming entitlement expires for the voice channel
     */
    hd_streaming_until?: string | null;
    /**
     * The ID of the user who applied the HD streaming entitlement to the voice channel
     */
    hd_streaming_buyer_id?: Snowflake | null;
    /**
     * The lobby linked to the channel
     */
    // linked_lobby?: LinkedLobby | null;
}

/**
 * Type 10, 11 and 12 are only available in API v9 and above.
 */
export enum ChannelType {
    /**
     * A text channel within a guild
     *
     * Value: 0
     * Name: GUILD_TEXT
     */
    GUILD_TEXT = 0,
    /**
     * A private channel between two users
     *
     * Value: 1
     * Name: DM
     */
    DM = 1,
    /**
     * A voice channel within a guild
     *
     * Value: 2
     * Name: GUILD_VOICE
     */
    GUILD_VOICE = 2,
    /**
     * A private channel between multiple users
     *
     * Value: 3
     * Name: GROUP_DM
     */
    GROUP_DM = 3,
    /**
     * An organizational category that contains up to 50 channels
     *
     * Value: 4
     * Name: GUILD_CATEGORY
     */
    GUILD_CATEGORY = 4,
    /**
     * Almost identical to GUILD_TEXT , a channel that users can follow and crosspost into their own guild
     *
     * Value: 5
     * Name: GUILD_NEWS
     */
    GUILD_NEWS = 5,
    /**
     * A channel in which developers can showcase their SKUs
     *
     * Value: 6
     * Name: GUILD_STORE
     */
    GUILD_STORE = 6,
    /**
     * A temporary sub-channel within a GUILD_NEWS channel
     *
     * Value: 10
     * Name: NEWS_THREAD
     */
    NEWS_THREAD = 10,
    /**
     * a temporary sub-channel within a GUILD_TEXT , GUILD_FORUM , or GUILD_MEDIA channel
     *
     * Value: 11
     * Name: PUBLIC_THREAD
     */
    PUBLIC_THREAD = 11,
    /**
     * a temporary sub-channel within a GUILD_TEXT channel that is only viewable by those invited and those with the MANAGE_THREADS permission
     *
     * Value: 12
     * Name: PRIVATE_THREAD
     */
    PRIVATE_THREAD = 12,
    /**
     * A voice channel for hosting events with an audience in a guild
     *
     * Value: 13
     * Name: GUILD_STAGE_VOICE
     */
    GUILD_STAGE_VOICE = 13,
    /**
     * The main channel in a hub containing the listed guilds
     *
     * Value: 14
     * Name: GUILD_DIRECTORY
     */
    GUILD_DIRECTORY = 14,
    /**
     * A channel that can only contain threads
     *
     * Value: 15
     * Name: GUILD_FORUM
     */
    GUILD_FORUM = 15,
    /**
     * A channel that can only contain threads in a gallery view
     *
     * Value: 16
     * Name: GUILD_MEDIA
     */
    GUILD_MEDIA = 16,
    /**
     * A game lobby channel
     *
     * Value: 17
     * Name: LOBBY
     */
    LOBBY = 17,
    /**
     * A private channel created by the social layer SDK
     *
     * Value: 18
     * Name: EPHEMERAL_DM
     */
    EPHEMERAL_DM = 18,
}
