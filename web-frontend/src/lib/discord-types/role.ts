import type { Snowflake } from "./common";

export interface Role {
    /**
     * The ID of the role
     */
    id: Snowflake;
    /**
     * The name of the role (max 100 characters)
     */
    name: string;
    /**
     * The description for the role (max 90 characters)
     */
    description: string | null;
    /**
     * The color of the role represented as an integer representation of a hexadecimal color code
     * @deprecated
     */
    color: number;
    /**
     * The colors of the role encoded as an integer representation of hexadecimal color codes
     */
    // colors: RoleColors;
    /**
     * Whether this role is pinned in the user listing
     */
    hoist: boolean;
    /**
     * The role's icon hash
     */
    icon?: string | null;
    /**
     * The role's unicode emoji
     */
    unicode_emoji?: string | null;
    /**
     * Position of this role
     */
    position: number;
    /**
     * The permission bitwise value for the role
     */
    permissions: string;
    /**
     * Whether this role is managed by an integration
     */
    managed: boolean;
    /**
     * Whether this role is mentionable
     */
    mentionable: boolean;
    /**
     * The role's flags
     */
    flags?: number;
    /**
     * The tags this role has
     */
    // tags?: RoleTags;
}

export function hasPermission(
    userPermissions: bigint,
    permission: bigint
): boolean {
    return (userPermissions & permission) === permission;
}
