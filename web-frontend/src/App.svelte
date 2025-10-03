<script lang="ts">
    import { Router, Route } from "svelte-routing";
    import Home from "./pages/Home.svelte";
    import Dashboard from "./pages/Dashboard.svelte";
    import "./app.css";
    import Redirect from "./lib/Redirect.svelte";
    import ManageGuild from "./pages/ManageGuild.svelte";
    import { baseUrl } from "./lib/backend-types";
    import { InviteUrl } from "./lib/discord-types";
</script>

<Router>
    <div>
        <Route path="/"><Home /></Route>
        <Route path="/dashboard"><Dashboard /></Route>
        <Route path="/invite"><Redirect to={InviteUrl} /></Route>
        <Route path="/login"><Redirect to="{baseUrl}/login" /></Route>
        <Route path="/manage/:id" let:params
            ><ManageGuild id={params.id} /></Route
        >
    </div>
</Router>
