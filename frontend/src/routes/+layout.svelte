<script>
	import '../app.css';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { getWebSocket, wsConnected } from '$lib/services/websocket';
	import { activeAlarms, alarmCounts, loadAllData } from '$lib/stores/scada';
	import { page } from '$app/stores';
	import ThemeToggle from '$lib/components/common/ThemeToggle.svelte';
	import StatusIndicator from '$lib/components/common/StatusIndicator.svelte';

	let currentPath = '';
	let sidebarOpen = true;
	let mobileMenuOpen = false;
	let notificationsOpen = false;
	let loginUsername = '';
	let loginPassword = '';
	let loginError = '';
	let loginLoading = false;
	let isMobile = false;

	$: currentPath = $page.url.pathname;
	$: totalAlarms = ($alarmCounts?.critical || 0) + ($alarmCounts?.warning || 0) + ($alarmCounts?.info || 0);
	$: recentAlarms = ($activeAlarms || []).slice(0, 5);

	const navItems = [
		{ path: '/', label: 'Dashboard', icon: 'dashboard' },
		{ path: '/water', label: 'Water System', icon: 'water' },
		{ path: '/solar', label: 'Solar System', icon: 'solar' },
		{ path: '/alarms', label: 'Alarms', icon: 'alarm' },
		{ path: '/history', label: 'History', icon: 'history' },
		{ path: '/settings', label: 'Settings', icon: 'settings' },
	];

	function isActive(itemPath) {
		if (itemPath === '/') return currentPath === '/';
		return currentPath.startsWith(itemPath);
	}

	onMount(async () => {
		// Check screen size
		checkMobile();
		window.addEventListener('resize', checkMobile);

		const authenticated = await auth.checkAuth();
		if (authenticated) {
			initApp();
		}

		return () => {
			window.removeEventListener('resize', checkMobile);
		};
	});

	function checkMobile() {
		isMobile = window.innerWidth < 768;
		if (isMobile) {
			sidebarOpen = false;
			mobileMenuOpen = false;
		} else {
			sidebarOpen = true;
		}
	}

	function initApp() {
		const ws = getWebSocket();
		ws.connect();
		loadAllData();

		const interval = setInterval(loadAllData, 30000);

		// Listen for new alarms to show notification
		ws.subscribe('alarm', (msg) => {
			loadAllData();
		});

		return () => {
			clearInterval(interval);
			ws.disconnect();
		};
	}

	async function handleLogin() {
		if (!loginUsername || !loginPassword) {
			loginError = 'Please enter username and password';
			return;
		}
		loginLoading = true;
		loginError = '';
		try {
			await auth.login(loginUsername, loginPassword);
			initApp();
		} catch (e) {
			loginError = e.message || 'Login failed';
		} finally {
			loginLoading = false;
		}
	}

	function handleLogout() {
		auth.logout();
		const ws = getWebSocket();
		ws.disconnect();
	}

	function toggleSidebar() {
		if (isMobile) {
			mobileMenuOpen = !mobileMenuOpen;
		} else {
			sidebarOpen = !sidebarOpen;
		}
	}

	function closeMobileMenu() {
		if (isMobile) mobileMenuOpen = false;
	}

	function toggleNotifications() {
		notificationsOpen = !notificationsOpen;
	}

	function closeNotifications() {
		notificationsOpen = false;
	}

	function formatAlarmTime(ts) {
		const d = new Date(ts);
		return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<svelte:window on:click={closeNotifications} />

{#if $auth.isAuthenticated}
	<div class="app-layout" class:sidebar-collapsed={!sidebarOpen && !isMobile}>
		<!-- Mobile overlay -->
		{#if mobileMenuOpen && isMobile}
			<div class="mobile-overlay" on:click={closeMobileMenu} on:keydown></div>
		{/if}

		<!-- Sidebar -->
		<aside class="sidebar" class:open={mobileMenuOpen || (!isMobile && sidebarOpen)} class:mobile={isMobile}>
			<div class="sidebar-header">
				<div class="logo-section">
					<h1 class="logo">SCADA</h1>
					<span class="logo-sub">Water & Solar Control</span>
				</div>
				{#if isMobile}
					<button class="close-menu-btn" on:click={closeMobileMenu}>
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
						</svg>
					</button>
				{/if}
			</div>

			<nav class="sidebar-nav">
				{#each navItems as item}
					<a
						href={item.path}
						class="nav-item"
						class:active={isActive(item.path)}
						on:click={closeMobileMenu}
					>
						<span class="nav-icon">
							{#if item.icon === 'dashboard'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
							{:else if item.icon === 'water'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/></svg>
							{:else if item.icon === 'solar'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
							{:else if item.icon === 'alarm'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
							{:else if item.icon === 'history'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
							{:else if item.icon === 'settings'}
								<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
							{/if}
						</span>
						<span class="nav-label">{item.label}</span>
						{#if item.icon === 'alarm' && ($alarmCounts?.critical || 0) > 0}
							<span class="alarm-count-badge">{$alarmCounts.critical}</span>
						{/if}
					</a>
				{/each}
			</nav>

			<div class="sidebar-footer">
				<div class="ws-status">
					<StatusIndicator
						status={$wsConnected ? 'connected' : 'disconnected'}
						label={$wsConnected ? 'Live' : 'Offline'}
						size="sm"
					/>
				</div>
				<div class="user-section">
					<div class="user-avatar">
						{($auth.user?.username || 'U').charAt(0).toUpperCase()}
					</div>
					<div class="user-details">
						<span class="user-name">{$auth.user?.username || 'User'}</span>
						<span class="user-role">{$auth.user?.role || 'viewer'}</span>
					</div>
				</div>
				<button class="logout-btn" on:click={handleLogout}>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
					<span>Logout</span>
				</button>
			</div>
		</aside>

		<!-- Main area -->
		<div class="main-wrapper">
			<!-- Top Header Bar -->
			<header class="top-header">
				<div class="header-left">
					<button class="menu-toggle" on:click={toggleSidebar}>
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
						</svg>
					</button>
					<div class="breadcrumb">
						{#each navItems as item}
							{#if isActive(item.path)}
								<span class="bc-label">{item.label}</span>
							{/if}
						{/each}
					</div>
				</div>

				<div class="header-right">
					<!-- Connection indicator -->
					<div class="header-ws-status" class:connected={$wsConnected}>
						<span class="ws-dot"></span>
						<span class="ws-text">{$wsConnected ? 'LIVE' : 'OFFLINE'}</span>
					</div>

					<!-- Notifications bell -->
					<div class="notification-wrapper" on:click|stopPropagation>
						<button class="notification-bell" on:click={toggleNotifications} class:has-alarms={totalAlarms > 0}>
							<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
								<path d="M13.73 21a2 2 0 0 1-3.46 0"/>
							</svg>
							{#if totalAlarms > 0}
								<span class="bell-badge">{totalAlarms}</span>
							{/if}
						</button>

						{#if notificationsOpen}
							<div class="notification-dropdown">
								<div class="notif-header">
									<span>Recent Alarms</span>
									<a href="/alarms" on:click={closeNotifications}>View All</a>
								</div>
								{#if recentAlarms.length > 0}
									{#each recentAlarms as alarm}
										<div class="notif-item" class:critical={alarm.severity === 'critical'}>
											<span class="notif-severity"
												class:sev-critical={alarm.severity === 'critical'}
												class:sev-warning={alarm.severity === 'warning'}
												class:sev-info={alarm.severity === 'info'}>
												{alarm.severity === 'critical' ? '!!' : alarm.severity === 'warning' ? '!' : 'i'}
											</span>
											<div class="notif-content">
												<div class="notif-message">{alarm.message}</div>
												<div class="notif-meta">
													<span class="notif-subsystem">{alarm.subsystem}</span>
													<span class="notif-time">{formatAlarmTime(alarm.occurred_at)}</span>
												</div>
											</div>
										</div>
									{/each}
								{:else}
									<div class="notif-empty">No active alarms</div>
								{/if}
							</div>
						{/if}
					</div>

					<!-- Theme toggle -->
					<ThemeToggle />

					<!-- User chip -->
					<div class="header-user">
						<span class="header-username">{$auth.user?.username}</span>
					</div>
				</div>
			</header>

			<!-- Main Content -->
			<main class="main-content">
				<slot />
			</main>
		</div>
	</div>

{:else}
	<!-- Login Screen -->
	<div class="login-screen">
		<div class="login-bg-pattern"></div>
		<div class="login-card">
			<div class="login-header">
				<h1 class="login-logo">SCADA</h1>
				<p class="login-subtitle">Water & Solar Control System</p>
			</div>

			<form on:submit|preventDefault={handleLogin} class="login-form">
				{#if loginError}
					<div class="login-error">
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>
						</svg>
						{loginError}
					</div>
				{/if}

				<div class="login-field">
					<label for="username">Username</label>
					<input
						id="username"
						type="text"
						bind:value={loginUsername}
						placeholder="Enter username"
						autocomplete="username"
						disabled={loginLoading}
					/>
				</div>

				<div class="login-field">
					<label for="password">Password</label>
					<input
						id="password"
						type="password"
						bind:value={loginPassword}
						placeholder="Enter password"
						autocomplete="current-password"
						disabled={loginLoading}
					/>
				</div>

				<button type="submit" class="login-submit" disabled={loginLoading}>
					{loginLoading ? 'Signing in...' : 'Sign In'}
				</button>
			</form>

			<div class="login-footer">
				<StatusIndicator status="online" label="System Operational" size="sm" />
			</div>
		</div>
	</div>
{/if}

<style>
	/* ─── Layout ─── */
	.app-layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	/* ─── Sidebar ─── */
	.sidebar {
		width: var(--sidebar-width);
		min-width: var(--sidebar-width);
		background: var(--bg-secondary);
		border-right: 1px solid var(--border-color);
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		transition: all 0.25s ease;
		z-index: 50;
	}
	.sidebar.mobile {
		position: fixed;
		top: 0;
		left: 0;
		bottom: 0;
		transform: translateX(-100%);
	}
	.sidebar.mobile.open {
		transform: translateX(0);
		box-shadow: 4px 0 24px rgba(0, 0, 0, 0.4);
	}
	.sidebar-collapsed .sidebar:not(.mobile) {
		width: 0;
		min-width: 0;
		overflow: hidden;
		border-right: none;
	}

	.sidebar-header {
		padding: 18px 16px;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.logo-section {
		display: flex;
		flex-direction: column;
	}
	.logo {
		font-size: 1.5rem;
		font-weight: 800;
		font-family: var(--font-mono);
		background: linear-gradient(135deg, var(--color-water), var(--color-solar));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		line-height: 1.2;
	}
	.logo-sub {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}
	.close-menu-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px;
	}

	.sidebar-nav {
		flex: 1;
		padding: 10px 8px;
	}
	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		border-radius: 8px;
		color: var(--text-secondary);
		text-decoration: none;
		font-size: 0.85rem;
		transition: all 0.15s;
		margin-bottom: 2px;
		position: relative;
	}
	.nav-item:hover {
		background: var(--bg-card);
		color: var(--text-primary);
	}
	.nav-item.active {
		background: rgba(59, 130, 246, 0.1);
		color: var(--color-info);
		font-weight: 600;
	}
	.nav-item.active::before {
		content: '';
		position: absolute;
		left: 0;
		top: 6px;
		bottom: 6px;
		width: 3px;
		background: var(--color-info);
		border-radius: 0 2px 2px 0;
	}
	.nav-icon {
		width: 20px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.nav-label {
		white-space: nowrap;
	}
	.alarm-count-badge {
		margin-left: auto;
		background: var(--color-danger);
		color: white;
		font-size: 0.6rem;
		font-weight: 700;
		padding: 1px 6px;
		border-radius: 10px;
		animation: pulse 1.5s infinite;
	}
	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.4; }
	}

	.sidebar-footer {
		padding: 12px 12px 16px;
		border-top: 1px solid var(--border-color);
	}
	.ws-status {
		margin-bottom: 12px;
		padding: 0 4px;
	}
	.user-section {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 4px;
		margin-bottom: 8px;
	}
	.user-avatar {
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: linear-gradient(135deg, var(--color-info), var(--color-water));
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 0.85rem;
		color: white;
		flex-shrink: 0;
	}
	.user-details {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.user-name {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.user-role {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.logout-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		width: 100%;
		padding: 8px;
		border-radius: 6px;
		background: transparent;
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.15s;
	}
	.logout-btn:hover {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
		color: #ef4444;
	}

	/* ─── Mobile overlay ─── */
	.mobile-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 40;
	}

	/* ─── Main wrapper ─── */
	.main-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-width: 0;
	}

	/* ─── Top Header ─── */
	.top-header {
		height: var(--header-height);
		min-height: var(--header-height);
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border-color);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 20px;
		gap: 16px;
	}
	.header-left {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.menu-toggle {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 6px;
		border-radius: 6px;
		display: flex;
		align-items: center;
	}
	.menu-toggle:hover {
		background: var(--bg-card);
		color: var(--text-primary);
	}
	.breadcrumb {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.header-ws-status {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px;
		border-radius: 20px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.2);
	}
	.header-ws-status.connected {
		background: rgba(34, 197, 94, 0.1);
		border-color: rgba(34, 197, 94, 0.2);
	}
	.ws-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #ef4444;
	}
	.header-ws-status.connected .ws-dot {
		background: #22c55e;
		box-shadow: 0 0 6px #22c55e;
		animation: pulse 2s infinite;
	}
	.ws-text {
		font-size: 0.6rem;
		font-weight: 700;
		letter-spacing: 0.06em;
		color: var(--text-secondary);
	}

	/* ─── Notification Bell ─── */
	.notification-wrapper {
		position: relative;
	}
	.notification-bell {
		position: relative;
		background: none;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 7px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
	}
	.notification-bell:hover {
		background: var(--bg-card);
		color: var(--text-primary);
	}
	.notification-bell.has-alarms {
		border-color: rgba(239, 68, 68, 0.3);
		color: var(--color-danger);
	}
	.bell-badge {
		position: absolute;
		top: -4px;
		right: -4px;
		background: #ef4444;
		color: white;
		font-size: 0.55rem;
		font-weight: 700;
		padding: 1px 4px;
		border-radius: 8px;
		min-width: 16px;
		text-align: center;
		line-height: 1.3;
	}

	.notification-dropdown {
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: 8px;
		width: 340px;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
		z-index: 100;
		overflow: hidden;
	}
	.notif-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-color);
		font-size: 0.8rem;
		font-weight: 600;
	}
	.notif-header a {
		font-size: 0.7rem;
		color: var(--color-info);
		text-decoration: none;
	}
	.notif-item {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 10px 16px;
		border-bottom: 1px solid var(--border-color);
		transition: background 0.1s;
	}
	.notif-item:hover {
		background: var(--bg-card-hover);
	}
	.notif-item.critical {
		background: rgba(239, 68, 68, 0.04);
	}
	.notif-severity {
		width: 24px;
		height: 24px;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 800;
		flex-shrink: 0;
		margin-top: 1px;
	}
	.sev-critical { background: rgba(239,68,68,0.15); color: #ef4444; }
	.sev-warning { background: rgba(245,158,11,0.15); color: #f59e0b; }
	.sev-info { background: rgba(59,130,246,0.15); color: #3b82f6; }
	.notif-content {
		flex: 1;
		min-width: 0;
	}
	.notif-message {
		font-size: 0.78rem;
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.notif-meta {
		display: flex;
		gap: 8px;
		margin-top: 2px;
	}
	.notif-subsystem {
		font-size: 0.6rem;
		font-weight: 600;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.notif-time {
		font-size: 0.6rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.notif-empty {
		padding: 24px;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.header-user {
		display: none;
	}
	@media (min-width: 768px) {
		.header-user {
			display: flex;
			align-items: center;
			gap: 6px;
		}
	}
	.header-username {
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	/* ─── Main Content ─── */
	.main-content {
		flex: 1;
		overflow-y: auto;
		padding: 20px;
	}
	@media (min-width: 768px) {
		.main-content {
			padding: 24px;
		}
	}

	/* ─── Login Screen ─── */
	.login-screen {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-primary);
		position: relative;
		overflow: hidden;
	}
	.login-bg-pattern {
		position: absolute;
		inset: 0;
		background-image:
			radial-gradient(circle at 25% 25%, rgba(6, 182, 212, 0.06) 0%, transparent 50%),
			radial-gradient(circle at 75% 75%, rgba(245, 158, 11, 0.06) 0%, transparent 50%);
	}
	.login-card {
		position: relative;
		width: 100%;
		max-width: 400px;
		margin: 0 16px;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 16px;
		padding: 40px 32px;
		box-shadow: 0 8px 40px rgba(0, 0, 0, 0.3);
	}
	.login-header {
		text-align: center;
		margin-bottom: 32px;
	}
	.login-logo {
		font-size: 2.5rem;
		font-weight: 800;
		font-family: var(--font-mono);
		background: linear-gradient(135deg, var(--color-water), var(--color-solar));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin-bottom: 6px;
	}
	.login-subtitle {
		font-size: 0.8rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.12em;
	}
	.login-form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.login-error {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.8rem;
	}
	.login-field {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.login-field label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-weight: 600;
	}
	.login-field input {
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 12px 14px;
		color: var(--text-primary);
		font-size: 0.9rem;
		transition: border-color 0.15s;
	}
	.login-field input:focus {
		outline: none;
		border-color: var(--color-info);
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
	}
	.login-field input::placeholder {
		color: var(--text-muted);
	}
	.login-submit {
		margin-top: 8px;
		padding: 12px;
		background: var(--color-info);
		border: none;
		border-radius: 8px;
		color: white;
		font-size: 0.9rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}
	.login-submit:hover {
		background: #2563eb;
	}
	.login-submit:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.login-footer {
		display: flex;
		justify-content: center;
		margin-top: 24px;
		padding-top: 16px;
		border-top: 1px solid var(--border-color);
	}

	@media (max-width: 768px) {
		.header-ws-status .ws-text {
			display: none;
		}
		.notification-dropdown {
			width: calc(100vw - 32px);
			right: -60px;
		}
	}
</style>
