<script lang="ts">
    import { get } from "../lib/backend-types";
    import { type Guild } from "../lib/discord-types";
    import NavBar from "../lib/NavBar.svelte";

    export let id;

    const guildPromise = get<Guild>(`/manage/${id}`);
</script>

<svelte:head>
    <title>Servers - Zayden</title>
</svelte:head>

<NavBar />

<main class="dashboard">
    {#await guildPromise then guild}
        <pre>{JSON.stringify(guild, null, 2)}</pre>
    {/await}
</main>
