<script>
	import '../app.css';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { getWebSocket, wsConnected } from '$lib/services/websocket';
	import { alarmCounts, loadAllData } from '$lib/stores/scada';
	import { page } from '$app/stores';

	let currentPath = '';

	$: currentPath = $page.url.pathname;

	const navItems = [
		{ path: '/', label: 'Dashboard', icon: '&#9632;' },
		{ path: '/water', label: 'Water System', icon: '&#9679;' },
		{ path: '/solar', label: 'Solar System', icon: '&#9788;' },
		{ path: '/alarms', label: 'Alarms', icon: '&#9888;' },
		{ path: '/history', label: 'History', icon: '&#8987;' },
		{ path: '/settings', label: 'Settings', icon: '&#9881;' },
	];

	onMount(async () => {
		const authenticated = await auth.checkAuth();
		if (authenticated) {
			const ws = getWebSocket();
			ws.connect();
			loadAllData();

			// Refresh data every 30 seconds
			const interval = setInterval(loadAllData, 30000);
			return () => {
				clearInterval(interval);
				ws.disconnect();
			};
		}
	});
</script>

{#if $auth.isAuthenticated}
	<div class="app-layout">
		<!-- Sidebar -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<h1 class="logo">SCADA</h1>
				<span class="logo-sub">Water & Solar</span>
			</div>

			<nav class="sidebar-nav">
				{#each navItems as item}
					<a
						href={item.path}
						class="nav-item"
						class:active={currentPath === item.path}
					>
						<span class="nav-icon">{@html item.icon}</span>
						<span>{item.label}</span>
						{#if item.path === '/alarms' && $alarmCounts.critical > 0}
							<span class="alarm-badge">{$alarmCounts.critical}</span>
						{/if}
					</a>
				{/each}
			</nav>

			<div class="sidebar-footer">
				<div class="connection-status">
					<span class="status-dot" class:online={$wsConnected} class:offline={!$wsConnected}></span>
					<span>{$wsConnected ? 'Connected' : 'Disconnected'}</span>
				</div>
				<div class="user-info">
					<span>{$auth.user?.username}</span>
					<span class="badge badge-info">{$auth.user?.role}</span>
				</div>
				<button class="btn" on:click={() => auth.logout()} style="width:100%; margin-top: 8px;">
					Logout
				</button>
			</div>
		</aside>

		<!-- Main Content -->
		<main class="main-content">
			<slot />
		</main>
	</div>
{:else}
	<slot />
{/if}

<style>
	.app-layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	.sidebar {
		width: var(--sidebar-width);
		min-width: var(--sidebar-width);
		background: var(--bg-secondary);
		border-right: 1px solid var(--border-color);
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.sidebar-header {
		padding: 20px 16px;
		border-bottom: 1px solid var(--border-color);
	}

	.logo {
		font-size: 1.5rem;
		font-weight: 800;
		font-family: var(--font-mono);
		background: linear-gradient(135deg, var(--color-water), var(--color-solar));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
	}

	.logo-sub {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.sidebar-nav {
		flex: 1;
		padding: 8px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		border-radius: 6px;
		color: var(--text-secondary);
		text-decoration: none;
		font-size: 0.875rem;
		transition: all 0.15s;
		margin-bottom: 2px;
	}
	.nav-item:hover {
		background: var(--bg-card);
		color: var(--text-primary);
	}
	.nav-item.active {
		background: var(--bg-card);
		color: var(--text-primary);
		border-left: 3px solid var(--color-info);
	}

	.nav-icon {
		width: 20px;
		text-align: center;
		font-size: 1rem;
	}

	.alarm-badge {
		margin-left: auto;
		background: var(--color-danger);
		color: white;
		font-size: 0.65rem;
		font-weight: 700;
		padding: 1px 6px;
		border-radius: 10px;
		animation: pulse 1.5s infinite;
	}

	.sidebar-footer {
		padding: 16px;
		border-top: 1px solid var(--border-color);
	}

	.connection-status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 12px;
	}

	.user-info {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 0.8rem;
		margin-bottom: 4px;
	}

	.main-content {
		flex: 1;
		overflow-y: auto;
		padding: 24px;
	}
</style>
