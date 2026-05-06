<script lang="ts">
    import { Router, Route } from "svelte-routing";
    import Home from "./pages/Home.svelte";
    import Dashboard from "./pages/Dashboard.svelte";
    import "./app.css";
    import Redirect from "./lib/Redirect.svelte";
    import ManageGuild from "./pages/ManageGuild.svelte";
    import { baseUrl } from "./lib/backend-types";
    import { INVITE_URL } from "./lib/discord-types/common";
</script>

<Router>
    <div>
        <Route path="/"><Home /></Route>
        <Route path="/dashboard"><Dashboard /></Route>
        <Route path="/invite"><Redirect to={INVITE_URL} /></Route>
        <Route path="/login"><Redirect to="{baseUrl}/login" /></Route>
        <Route path="/manage/:id" let:params
            ><ManageGuild id={params.id} /></Route
        >
    </div>
</Router>
