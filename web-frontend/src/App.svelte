<script lang="ts">
  import { Router, Route } from "svelte-routing";
  import Home from "./pages/Home.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import "./app.css";
  import Redirect from "./lib/Redirect.svelte";
  import { BASE_URL, discordInviteUrl } from "./lib/variables";
</script>

<Router>
  <div>
    <Route path="/"><Home /></Route>
    <Route path="/dashboard"><Dashboard /></Route>
    <Route path="/invite"><Redirect to={discordInviteUrl} /></Route>
    <Route path="/login"><Redirect to="{BASE_URL}/login" /></Route>
  </div>
</Router>
