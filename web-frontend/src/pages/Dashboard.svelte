<script lang="ts">
    import { onMount } from "svelte";
    import NavBar from "../lib/NavBar.svelte";
    import serversIcon from "../assets/servers_icon.svg";
    import settingsIcon from "../assets/settings_icon.svg";
    import * as vars from "../lib/variables";
    import { navigate } from "svelte-routing";
    import Cookies from "js-cookie";
    import { avatar, icon, type Guild, type User } from "../discord-types";

    function authFail(): never {
        console.log("No auth token, redirecting to login");
        navigate("/login");
        throw new Error("authFail");
    }

    async function user(authToken: string): Promise<User> {
        const response = await fetch(`${vars.discordBaseUrl}/users/@me`, {
            headers: { authorization: `Bearer ${authToken}` },
        });

        if (response.status == 401) {
            authFail();
        }

        return await response.json();
    }

    async function guilds(authToken: string): Promise<Guild[]> {
        const response = await fetch(
            `${vars.discordBaseUrl}/users/@me/guilds`,
            {
                headers: { authorization: `Bearer ${authToken}` },
            },
        );

        return await response.json();
    }

    const authToken = Cookies.get("auth-token");

    if (!authToken) {
        authFail();
    }

    const userPromise = user(authToken);
    const guildsPromise = guilds(authToken);
</script>

<svelte:head>
    <title>Servers - Zayden</title>
</svelte:head>

<NavBar />

<main class="dashboard">
    {#await userPromise then user}
        <header class="profile-header">
            <img src={avatar(user)} alt="User Avatar" class="avatar" />
            <div class="user-details">
                <h1 class="username">{user.global_name}</h1>
                <p class="handle">{user.username}</p>
            </div>
        </header>
    {/await}

    <!-- Navigation Tabs -->
    <nav class="main-nav">
        <button class="nav-button active">
            <img src={serversIcon} height="16" width="16" alt="Servers Icon" />
            Servers
        </button>

        <button class="nav-button">
            <img
                src={settingsIcon}
                height="16"
                width="16"
                alt="Settings Icon"
            />
            Settings
        </button>
    </nav>

    <!-- Servers Grid -->
    <div class="servers-section">
        {#await guildsPromise then guilds}
            <div class="servers-header">
                <h2>
                    Servers
                    <small class="server-count">({guilds.length})</small>
                </h2>
            </div>

            <div class="server-grid">
                {#each guilds as guild}
                    <div class="server-card">
                        <img
                            src={icon(guild)}
                            alt="{guild.name} icon"
                            class="server-icon"
                        />
                        <p class="server-name">{guild.name}</p>
                    </div>
                {/each}
            </div>
        {/await}
    </div>
</main>

<style>
    :root {
        --font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
            Helvetica, Arial, sans-serif;
        --bg-surface: #27282b;
        --bg-nav-inactive: #2c2d31;
        --bg-nav-active: #1c1d22; /* Same as main background to blend in */
        --border-color: #3a3b40;
        --text-primary: #eaeaea;
        --text-secondary: #8e9297;
        --text-handle: #e44b23;
        --premium-color: #e5a443;
        --settings-color: #30a46c;
    }

    .dashboard {
        color: var(--text-primary);
        font-family: var(--font-family);
        padding: 24px;
        border-radius: 8px;
        max-width: 800px;
        width: 100%;
        margin: 0 auto;
        margin-top: 80px;
    }

    /* Profile Header */
    .profile-header {
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .avatar {
        width: 80px;
        height: 80px;
        border-radius: 50%;
        border: 3px solid var(--bg-surface);
    }

    .username {
        font-size: 2.25rem;
        font-weight: 600;
        margin: 0;
    }

    .handle {
        font-size: 1rem;
        color: var(--text-handle);
        margin: 0;
    }

    /* Navigation */
    .main-nav {
        display: flex;
        margin-top: 24px;
        background-color: var(--bg-nav-inactive);
        border-radius: 8px;
        overflow: hidden;
        border: 1px solid var(--border-color);
    }

    .nav-button {
        flex: 1;
        padding: 12px 16px;
        background-color: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.95rem;
        font-weight: 500;
        cursor: pointer;
        transition:
            background-color 0.2s,
            color 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
    }

    .nav-button:not(:last-child) {
        border-right: 1px solid var(--border-color);
    }

    .nav-button.active {
        background-color: var(--bg-nav-active);
        color: var(--text-primary);
        border-bottom: 1px solid var(--bg-nav-active); /* Hide bottom border */
        margin-bottom: -1px; /* Pull content up */
    }

    .nav-button:not(.active):hover {
        background-color: #3a3b40;
    }

    .nav-button:nth-child(2) img {
        stroke: var(--settings-color);
    }

    .nav-button img {
        filter: invert(100%) brightness(2);
    }

    /* Servers Section */
    .servers-section {
        background-color: var(--color-bg-nav);
        margin-top: 5px;
        padding: 24px;
        border-radius: 8px;
    }

    .servers-header h2 {
        font-size: 2rem;
        font-weight: 600;
        margin: 0;
    }

    .server-count {
        color: var(--text-secondary);
    }

    /* Grid */
    .server-grid {
        margin-top: 24px;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 24px;
    }

    .server-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
        text-align: center;
    }

    .server-icon {
        width: 100%;
        aspect-ratio: 1 / 1;
        border-radius: 16px;
        object-fit: cover;
        background-color: var(--bg-surface);
    }

    .server-name {
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-secondary);
    }
</style>
