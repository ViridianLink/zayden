<script lang="ts">
    import { get } from "../lib/backend-types";
    import { ChannelType, type Channel } from "../lib/discord-types/channel";
    import { icon, type Guild } from "../lib/discord-types/guild";
    import NavBar from "../lib/NavBar.svelte";

    export let id;

    function partition_channels(
        channels: Channel[],
    ): [Channel[], Channel[], Channel[]] {
        return channels.reduce<[Channel[], Channel[], Channel[]]>(
            ([categories, text, voice], channel) => {
                switch (channel.type) {
                    case ChannelType.GUILD_CATEGORY:
                        categories.push(channel);
                        break;

                    case ChannelType.GUILD_MEDIA:
                    case ChannelType.GUILD_FORUM:
                    case ChannelType.GUILD_NEWS:
                    case ChannelType.GUILD_TEXT:
                        text.push(channel);
                        break;

                    case ChannelType.GUILD_STAGE_VOICE:
                    case ChannelType.GUILD_VOICE:
                        voice.push(channel);
                        break;
                }

                return [categories, text, voice];
            },
            [[], [], []],
        );
    }

    const guildPromise = get<Guild>(`/guild/${id}`);
    const channelsPromise = get<Channel[]>(`/guild/${id}/channels`);
</script>

<svelte:head>
    {#await guildPromise then guild}
        <title>{guild.name} - Zayden</title>
    {/await}
</svelte:head>

<NavBar />

<main class="container server-info">
    {#await guildPromise then guild}
        <header class="server-header">
            <img class="server-logo" src={icon(guild)} alt="Server Logo" />
            <h1 class="server-title">{guild.name}</h1>
        </header>

        <!-- Server Info Section -->
        <section class="server-stats">
            <h2 class="section-title">SERVER INFO</h2>
            <div class="stats-grid">
                <div class="stat-card">
                    <p class="stat-label">Members</p>
                    <p class="stat-value">
                        {guild.approximate_member_count ?? 0}
                    </p>
                </div>

                {#await channelsPromise then channels}
                    {@const [categories, text, voice] =
                        partition_channels(channels)}

                    <div class="stat-card">
                        <p class="stat-label">Categories</p>
                        <p class="stat-value">
                            {categories.length}
                        </p>
                    </div>

                    <div class="stat-card">
                        <p class="stat-label">Text Channels</p>
                        <p class="stat-value">{text.length}</p>
                    </div>

                    <div class="stat-card">
                        <p class="stat-label">Voice Channels</p>
                        <p class="stat-value">{voice.length}</p>
                    </div>
                {/await}
                <div class="stat-card">
                    <p class="stat-label">Roles</p>
                    <p class="stat-value">{guild.roles.length}</p>
                </div>
            </div>
        </section>
    {/await}
</main>

<style>
    .server-info {
        padding: 2rem 0;
        color: var(--color-text-main);
    }

    .server-header {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-bottom: 2rem;
    }

    .server-logo {
        width: 48px;
        height: 48px;
        border-radius: 0.5rem;
    }

    .server-title {
        font-size: 1.75rem;
        font-weight: 800;
        text-transform: uppercase;
        color: var(--color-text-main);
    }

    .section-title {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 1.5rem;
        color: var(--color-text-body);
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
        gap: 1.5rem;
    }

    .stat-card {
        padding: 1rem;
        border: 1px solid var(--color-border);
        border-radius: 0.75rem;
        background-color: var(--color-secondary-button-bg);
        text-align: center;
    }

    .stat-label {
        font-size: 0.875rem;
        color: var(--color-text-muted);
        margin-bottom: 0.5rem;
    }

    .stat-value {
        font-size: 1.25rem;
        font-weight: 700;
        color: var(--color-text-main);
    }
</style>
