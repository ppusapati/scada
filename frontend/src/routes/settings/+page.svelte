<script>
	import { auth, isAdmin } from '$lib/stores/auth';
	import { wsConnected } from '$lib/services/websocket';

	$: user = $auth.user;
</script>

<div class="settings-page">
	<header class="page-header">
		<h2>Settings</h2>
	</header>

	<div class="grid-2">
		<!-- User Profile -->
		<div class="card settings-section">
			<h3>User Profile</h3>
			{#if user}
				<div class="setting-row">
					<span class="setting-label">Username</span>
					<span class="setting-value">{user.username}</span>
				</div>
				<div class="setting-row">
					<span class="setting-label">Email</span>
					<span class="setting-value">{user.email}</span>
				</div>
				<div class="setting-row">
					<span class="setting-label">Role</span>
					<span class="badge badge-info">{user.role}</span>
				</div>
			{/if}
		</div>

		<!-- System Status -->
		<div class="card settings-section">
			<h3>System Status</h3>
			<div class="setting-row">
				<span class="setting-label">WebSocket</span>
				<span class="status-dot" class:online={$wsConnected} class:offline={!$wsConnected}></span>
				<span class="setting-value">{$wsConnected ? 'Connected' : 'Disconnected'}</span>
			</div>
			<div class="setting-row">
				<span class="setting-label">API Server</span>
				<span class="setting-value mono">/api/v1</span>
			</div>
			<div class="setting-row">
				<span class="setting-label">Version</span>
				<span class="setting-value mono">1.0.0</span>
			</div>
		</div>

		<!-- MQTT Topics Reference -->
		<div class="card settings-section" style="grid-column: span 2;">
			<h3>MQTT Topic Reference</h3>
			<table class="ref-table">
				<thead>
					<tr><th>Topic Pattern</th><th>Direction</th><th>Description</th></tr>
				</thead>
				<tbody>
					<tr><td class="mono">scada/water/{'{device_id}'}/telemetry</td><td>Device → Server</td><td>Water sensor readings</td></tr>
					<tr><td class="mono">scada/water/{'{device_id}'}/status</td><td>Device → Server</td><td>Device status changes</td></tr>
					<tr><td class="mono">scada/water/{'{device_id}'}/command</td><td>Server → Device</td><td>Control commands</td></tr>
					<tr><td class="mono">scada/solar/{'{device_id}'}/telemetry</td><td>Device → Server</td><td>Solar panel readings</td></tr>
					<tr><td class="mono">scada/solar/{'{device_id}'}/status</td><td>Device → Server</td><td>Inverter/panel status</td></tr>
					<tr><td class="mono">scada/solar/{'{device_id}'}/command</td><td>Server → Device</td><td>Inverter commands</td></tr>
					<tr><td class="mono">scada/system/heartbeat</td><td>Server</td><td>Backend heartbeat</td></tr>
				</tbody>
			</table>
		</div>
	</div>
</div>

<style>
	.settings-page { max-width: 1200px; margin: 0 auto; }
	.page-header { margin-bottom: 24px; }

	.settings-section { padding: 20px; }
	.settings-section h3 { font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 16px; padding-bottom: 8px; border-bottom: 1px solid var(--border-color); }

	.setting-row { display: flex; align-items: center; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border-color); }
	.setting-row:last-child { border-bottom: none; }
	.setting-label { font-size: 0.8rem; color: var(--text-muted); min-width: 100px; }
	.setting-value { font-size: 0.85rem; }

	.ref-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.ref-table th { text-align: left; padding: 8px; font-size: 0.7rem; text-transform: uppercase; color: var(--text-muted); border-bottom: 1px solid var(--border-color); }
	.ref-table td { padding: 8px; border-bottom: 1px solid var(--border-color); }
	.mono { font-family: var(--font-mono); font-size: 0.75rem; }
</style>
