<script lang="ts">
	import NavBar from "../lib/NavBar.svelte";
</script>

<svelte:head>
	<title>Home - Zayden</title>
</svelte:head>

<NavBar />

<main id="main-content">
	<div class="hero">
		<h1 class="hero__heading">
			<span class="gradient-text">Modern Bot for Modern Servers</span>
		</h1>

		<p class="hero__subtitle">
			Easily manage events, create temporary voice channels, handle
			support tickets, and play balanced interactive games - all from a
			clean and intuitive web dashboard.<br />Already trusted by growing
			communities, we invite you to try it out and make your server
			smarter, smoother, and more fun.
		</p>

		<div class="hero__actions">
			<!-- IMPORTANT: Replace '#' with your bot's actual invite link -->
			<a href="/invite" class="button button--primary button--large">
				<i class="fas fa-plus"></i>Add to Server
			</a>
			<a href="/login" class="button button--secondary button--large">
				<i class="fab fa-discord"></i>Login to Dashboard
			</a>
		</div>
	</div>
</main>
