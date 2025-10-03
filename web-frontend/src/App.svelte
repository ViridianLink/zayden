<script lang="ts">
  import { Router, Route } from "svelte-routing";
  import Home from "./pages/Home.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import "./app.css";
  import Redirect from "./lib/Redirect.svelte";
  import * as vars from "./lib/variables";
</script>

<Router>
  <div>
    <Route path="/"><Home /></Route>
    <Route path="/dashboard"><Dashboard /></Route>
    <Route path="/invite"><Redirect to={vars.discordInviteUrl} /></Route>
    <Route path="/login"><Redirect to="{vars.BASE_URL}/login" /></Route>
  </div>
</Router>
