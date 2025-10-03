<script lang="ts">
    import { get } from "../lib/backend-types";
    import { icon, type Guild } from "../lib/discord-types";
    import NavBar from "../lib/NavBar.svelte";

    export let id;

    const guildPromise = get<Guild>(`/manage/${id}`);
</script>

<svelte:head>
    <title>Servers - Zayden</title>
</svelte:head>

<NavBar />

<main class="container server-info">
    {#await guildPromise then guild}
        <!-- <pre>{JSON.stringify(guild, null, 2)}</pre> -->

        <!-- Header -->
        <header class="server-header">
            <img class="server-logo" src={icon(guild)} alt="Server Logo" />
            <h1 class="server-title">{guild.name}</h1>
        </header>

        <!-- Server Info Section -->
        <section class="server-stats">
            <h2 class="section-title">SERVER INFO</h2>
            <div class="stats-grid">
                <!-- {#each Object.entries(server.stats) as [label, value]}
                    <div class="stat-card">
                        <p class="stat-label">{label}</p>
                        <p class="stat-value">{value}</p>
                    </div>
                {/each} -->
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
