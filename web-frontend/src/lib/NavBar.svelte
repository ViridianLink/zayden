<script lang="ts">
	import { Link, navigate } from "svelte-routing";
	import userCircle from "../assets/user_circle.svg";
	import logo from "../assets/logo.png";
	import Cookies from "js-cookie";

	let isMobileMenuOpen = false;

	function toggleMobileMenu() {
		isMobileMenuOpen = !isMobileMenuOpen;
	}

	// Close mobile menu when a link is clicked
	function handleLinkClick() {
		isMobileMenuOpen = false;
	}

	const authToken = Cookies.get("auth-token");
</script>

<nav class="navbar">
	<div class="container">
		<div class="navbar__inner">
			<div class="navbar__brand">
				<img class="navbar__logo" src={logo} alt="Zayden Logo" />
				<span class="navbar__name">Zayden</span>

				<div class="navbar__links">
					<Link to="/">
						<div class="link">Home</div>
					</Link>
					<a
						href="https://discord.gg/sMHquCbPbv"
						class="link"
						target="_blank"
						rel="noopener noreferrer"
					>
						Join Our Discord
					</a>
					<Link to="/commands">
						<div class="link">Commands</div>
					</Link>
					<Link to="/premium" class="link premium">
						<div class="link">Get Premium</div>
					</Link>
				</div>
			</div>

			<!-- Right Side: Buttons -->
			<div class="navbar__actions">
				<Link
					to="/invite"
					class="button button--secondary button--small"
				>
					Add to Server
				</Link>

				{#if authToken}
					<Link
						to="/dashboard"
						class="button button--primary button--small"
						style="margin-left: 1rem;"
					>
						<img
							src={userCircle}
							height="24"
							width="24"
							alt="User Circle"
							id="user-circle"
						/>
						Dashboard
					</Link>
				{:else}
					<Link
						to="/login"
						class="button button--primary button--small"
						style="margin-left: 1rem;"
					>
						<i class="fab fa-discord"></i>Login
					</Link>
				{/if}
			</div>

			<!-- Mobile Menu Button -->
			<div class="mobile-menu__toggle">
				<button
					on:click={toggleMobileMenu}
					type="button"
					class="mobile-menu__button"
				>
					<span class="sr-only">Open main menu</span>
					{#if isMobileMenuOpen}
						<i class="fas fa-times h-6 w-6"></i>
					{:else}
						<i class="fas fa-bars h-6 w-6"></i>
					{/if}
				</button>
			</div>
		</div>
	</div>

	<!-- Mobile Menu, controlled by Svelte -->
	<div class:hidden={!isMobileMenuOpen} id="mobile-menu">
		<div class="mobile-menu__links">
			<Link to="/" class="link" on:click={handleLinkClick}>Home</Link>
			<a
				href="https://discord.gg/sMHquCbPbv"
				class="link"
				target="_blank"
				rel="noopener noreferrer"
			>
				Join Discord
			</a>
			<Link to="/commands" class="link" on:click={handleLinkClick}
				>Commands</Link
			>
			<a href="/premium" class="link premium">Premium</a>
		</div>
		<div class="button-container">
			<Link
				to="/invite"
				class="button button--secondary button--small"
				on:click={handleLinkClick}
			>
				Add to Server
			</Link>
			<Link
				to="/login"
				class="button button--primary button--small"
				on:click={handleLinkClick}
			>
				<i class="fab fa-discord"></i> Login
			</Link>
		</div>
	</div>
</nav>

<style>
	:root {
		--color-bg-nav: rgba(8, 12, 26, 0.75);
	}

	.navbar {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 50;
		background-color: var(--color-bg-nav);
		backdrop-filter: blur(10px);
		border-bottom: 1px solid rgba(55, 65, 81, 0.5);
		/* border-gray-700/50 */
	}

	.navbar__inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 4rem;
		/* 64px */
	}

	.navbar__brand {
		display: flex;
		align-items: center;
	}

	.navbar__logo {
		height: 2rem;
		width: 2rem;
	}

	.navbar__name {
		font-size: 1.25rem;
		font-weight: 700;
		margin-left: 0.75rem;
	}

	.navbar__links {
		display: none;
		/* Hidden on mobile */
		margin-left: 2.5rem;
	}

	.navbar__actions {
		display: none;
		/* Hidden on mobile */
		align-items: center;
		margin-left: 1.5rem;
	}

	@media (min-width: 768px) {
		/* "md" breakpoint */
		.navbar__links,
		.navbar__actions {
			display: flex;
		}

		.mobile-menu__toggle {
			display: none;
		}
	}

	.link {
		color: var(--color-text-body);
		padding: 0.5rem 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		margin: 0 0.25rem;
	}

	.link:hover {
		background-color: var(--color-secondary-button-bg-hover);
		color: var(--color-text-main);
	}

	.premium {
		color: var(--color-text-premium);
	}
</style>
