import { discordImageUrl } from "./lib/variables";

export type Snowflake = string;

export interface User {
    id: Snowflake
    username: string,
    global_name: string | null,
    avatar: string | null
}

export function display_name(user: User): string {
    return user.global_name ?? user.username
}

export function avatar(user: User): string {
    return `${discordImageUrl}/avatars/${user.id}/${user.avatar}.webp`
}

export interface Guild {
    id: Snowflake,
    name: string,
    icon: string | null,
}

export function icon(guild: Guild): string {
    return `${discordImageUrl}/icons/${guild.id}/${guild.icon}.png`
}