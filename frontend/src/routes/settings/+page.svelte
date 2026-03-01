<script>
	import { auth, isAdmin, isOperator } from '$lib/stores/auth';
	import { wsConnected, getWebSocket } from '$lib/services/websocket';
	import StatusIndicator from '$lib/components/common/StatusIndicator.svelte';

	$: user = $auth.user;

	// Setpoint configurations (editable)
	let setpoints = [
		{ id: 1, name: 'Tank Level High', subsystem: 'water', parameter: 'tank_level', value: 90, unit: '%', editing: false },
		{ id: 2, name: 'Tank Level Low', subsystem: 'water', parameter: 'tank_level_low', value: 15, unit: '%', editing: false },
		{ id: 3, name: 'Pressure High Alarm', subsystem: 'water', parameter: 'pressure_high', value: 6.5, unit: 'bar', editing: false },
		{ id: 4, name: 'Pressure Critical', subsystem: 'water', parameter: 'pressure_critical', value: 8.0, unit: 'bar', editing: false },
		{ id: 5, name: 'pH Low Limit', subsystem: 'water', parameter: 'ph_low', value: 6.5, unit: 'pH', editing: false },
		{ id: 6, name: 'pH High Limit', subsystem: 'water', parameter: 'ph_high', value: 8.5, unit: 'pH', editing: false },
		{ id: 7, name: 'Turbidity Alarm', subsystem: 'water', parameter: 'turbidity_high', value: 4.0, unit: 'NTU', editing: false },
		{ id: 8, name: 'Chlorine Low', subsystem: 'water', parameter: 'chlorine_low', value: 0.2, unit: 'mg/L', editing: false },
		{ id: 9, name: 'Inverter Overtemp', subsystem: 'solar', parameter: 'inverter_temp_high', value: 75, unit: 'C', editing: false },
		{ id: 10, name: 'Panel Overtemp', subsystem: 'solar', parameter: 'panel_temp_high', value: 80, unit: 'C', editing: false },
		{ id: 11, name: 'Grid Frequency Low', subsystem: 'solar', parameter: 'grid_freq_low', value: 49.5, unit: 'Hz', editing: false },
		{ id: 12, name: 'Grid Frequency High', subsystem: 'solar', parameter: 'grid_freq_high', value: 50.5, unit: 'Hz', editing: false },
	];

	let editValue = 0;
	let setpointSaveStatus = '';
	let setpointTimeout;

	function startEdit(sp) {
		if (!$isOperator) return;
		setpoints = setpoints.map(s => ({ ...s, editing: s.id === sp.id }));
		editValue = sp.value;
	}

	function cancelEdit() {
		setpoints = setpoints.map(s => ({ ...s, editing: false }));
	}

	function saveSetpoint(sp) {
		setpoints = setpoints.map(s => s.id === sp.id ? { ...s, value: editValue, editing: false } : s);
		setpointSaveStatus = `Setpoint "${sp.name}" updated to ${editValue} ${sp.unit}`;
		if (setpointTimeout) clearTimeout(setpointTimeout);
		setpointTimeout = setTimeout(() => { setpointSaveStatus = ''; }, 4000);
	}

	// Device management
	let managedDevices = [
		{ id: 'water-tank-001', name: 'Main Water Tank', type: 'Tank Sensor', subsystem: 'water', status: 'online', lastSeen: '2 min ago' },
		{ id: 'water-pump-001', name: 'Feed Pump #1', type: 'Pump Controller', subsystem: 'water', status: 'online', lastSeen: '1 min ago' },
		{ id: 'water-valve-001', name: 'Inlet Valve', type: 'Valve Actuator', subsystem: 'water', status: 'online', lastSeen: '30 sec ago' },
		{ id: 'water-ph-001', name: 'pH Sensor', type: 'Water Quality', subsystem: 'water', status: 'online', lastSeen: '45 sec ago' },
		{ id: 'solar-inv-001', name: 'Inverter Alpha', type: 'Grid Inverter', subsystem: 'solar', status: 'online', lastSeen: '15 sec ago' },
		{ id: 'solar-panel-001', name: 'Panel Array 1-8', type: 'PV Strings', subsystem: 'solar', status: 'online', lastSeen: '20 sec ago' },
		{ id: 'solar-panel-002', name: 'Panel Array 9-16', type: 'PV Strings', subsystem: 'solar', status: 'fault', lastSeen: '5 min ago' },
		{ id: 'solar-weather-001', name: 'Weather Station', type: 'Irradiance/Temp', subsystem: 'solar', status: 'online', lastSeen: '10 sec ago' },
	];

	// Notification preferences
	let notifications = {
		emailCritical: true,
		emailWarning: true,
		emailInfo: false,
		soundAlarm: true,
		browserNotify: false,
		dailyReport: true,
	};

	let notifSaveStatus = '';
	let notifTimeout;

	function saveNotifications() {
		notifSaveStatus = 'Notification preferences saved successfully.';
		if (notifTimeout) clearTimeout(notifTimeout);
		notifTimeout = setTimeout(() => { notifSaveStatus = ''; }, 4000);
	}

	// Tab management
	let activeTab = 'setpoints';
	const tabs = [
		{ id: 'setpoints', label: 'Setpoints', icon: 'sliders' },
		{ id: 'devices', label: 'Devices', icon: 'cpu' },
		{ id: 'profile', label: 'Profile', icon: 'user' },
		{ id: 'notifications', label: 'Notifications', icon: 'bell' },
		{ id: 'system', label: 'System Info', icon: 'info' },
	];
</script>

<div class="settings-page">
	<header class="page-header">
		<h2>
			<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:inline-block; vertical-align:middle; margin-right:8px; color:var(--text-secondary)">
				<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
			</svg>
			Settings
		</h2>
	</header>

	<!-- Tab Navigation -->
	<div class="tabs">
		{#each tabs as tab}
			<button
				class="tab-btn"
				class:active={activeTab === tab.id}
				on:click={() => activeTab = tab.id}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<!-- Tab Content -->
	<div class="tab-content">

		<!-- Setpoints Tab -->
		{#if activeTab === 'setpoints'}
			<div class="section">
				<div class="section-header">
					<h3>Setpoint Configuration</h3>
					<span class="section-desc">Configure alarm thresholds and operating parameters</span>
				</div>

				{#if setpointSaveStatus}
					<div class="save-status success">
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
						{setpointSaveStatus}
					</div>
				{/if}

				<div class="setpoint-table-wrap">
					<table class="setpoint-table">
						<thead>
							<tr>
								<th>Parameter</th>
								<th>Subsystem</th>
								<th>Current Value</th>
								<th>Unit</th>
								{#if $isOperator}
									<th>Actions</th>
								{/if}
							</tr>
						</thead>
						<tbody>
							{#each setpoints as sp}
								<tr>
									<td class="sp-name">{sp.name}</td>
									<td>
										<span class="subsystem-tag" class:water={sp.subsystem === 'water'} class:solar={sp.subsystem === 'solar'}>
											{sp.subsystem}
										</span>
									</td>
									<td class="sp-value-cell">
										{#if sp.editing}
											<input
												type="number"
												class="sp-input"
												bind:value={editValue}
												step="0.1"
												on:keydown={(e) => { if (e.key === 'Enter') saveSetpoint(sp); if (e.key === 'Escape') cancelEdit(); }}
											/>
										{:else}
											<span class="sp-value">{sp.value}</span>
										{/if}
									</td>
									<td class="sp-unit">{sp.unit}</td>
									{#if $isOperator}
										<td class="sp-actions">
											{#if sp.editing}
												<button class="sp-btn save" on:click={() => saveSetpoint(sp)}>Save</button>
												<button class="sp-btn cancel" on:click={cancelEdit}>Cancel</button>
											{:else}
												<button class="sp-btn edit" on:click={() => startEdit(sp)}>Edit</button>
											{/if}
										</td>
									{/if}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>

		<!-- Devices Tab -->
		{:else if activeTab === 'devices'}
			<div class="section">
				<div class="section-header">
					<h3>Device Management</h3>
					<span class="section-desc">View and manage connected SCADA devices</span>
				</div>

				<div class="device-list">
					{#each managedDevices as device}
						<div class="device-card" class:fault={device.status === 'fault'}>
							<div class="dc-main">
								<div class="dc-icon" class:water={device.subsystem === 'water'} class:solar={device.subsystem === 'solar'}>
									{#if device.subsystem === 'water'}
										<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/></svg>
									{:else}
										<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/></svg>
									{/if}
								</div>
								<div class="dc-info">
									<div class="dc-name">{device.name}</div>
									<div class="dc-meta">
										<span class="dc-type">{device.type}</span>
										<span class="dc-id">{device.id}</span>
									</div>
								</div>
							</div>
							<div class="dc-right">
								<StatusIndicator status={device.status} size="sm" />
								<span class="dc-lastseen">{device.lastSeen}</span>
							</div>
						</div>
					{/each}
				</div>
			</div>

		<!-- Profile Tab -->
		{:else if activeTab === 'profile'}
			<div class="section">
				<div class="section-header">
					<h3>User Profile</h3>
					<span class="section-desc">Your account information</span>
				</div>

				{#if user}
					<div class="profile-card">
						<div class="profile-avatar">
							{(user.username || 'U').charAt(0).toUpperCase()}
						</div>
						<div class="profile-details">
							<div class="pd-row">
								<span class="pd-label">Username</span>
								<span class="pd-value">{user.username}</span>
							</div>
							<div class="pd-row">
								<span class="pd-label">Email</span>
								<span class="pd-value">{user.email}</span>
							</div>
							<div class="pd-row">
								<span class="pd-label">Role</span>
								<span class="pd-value">
									<span class="role-badge" class:admin={user.role === 'admin'} class:operator={user.role === 'operator'} class:viewer={user.role === 'viewer'}>
										{user.role}
									</span>
								</span>
							</div>
							<div class="pd-row">
								<span class="pd-label">Account Status</span>
								<span class="pd-value">
									<StatusIndicator status={user.active ? 'online' : 'offline'} label={user.active ? 'Active' : 'Disabled'} />
								</span>
							</div>
							<div class="pd-row">
								<span class="pd-label">Permissions</span>
								<span class="pd-value">
									{#if user.role === 'admin'}
										Full system access, user management, configuration
									{:else if user.role === 'operator'}
										View data, control equipment, acknowledge alarms
									{:else}
										View-only access to dashboards and data
									{/if}
								</span>
							</div>
						</div>
					</div>
				{/if}
			</div>

		<!-- Notifications Tab -->
		{:else if activeTab === 'notifications'}
			<div class="section">
				<div class="section-header">
					<h3>Notification Preferences</h3>
					<span class="section-desc">Configure how you receive alarm notifications</span>
				</div>

				{#if notifSaveStatus}
					<div class="save-status success">
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
						{notifSaveStatus}
					</div>
				{/if}

				<div class="notif-form">
					<div class="notif-group">
						<h4>Email Notifications</h4>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.emailCritical} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Critical alarm emails</span>
						</label>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.emailWarning} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Warning alarm emails</span>
						</label>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.emailInfo} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Info alarm emails</span>
						</label>
					</div>

					<div class="notif-group">
						<h4>In-App Notifications</h4>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.soundAlarm} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Audible alarm sound</span>
						</label>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.browserNotify} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Browser push notifications</span>
						</label>
					</div>

					<div class="notif-group">
						<h4>Reports</h4>
						<label class="notif-toggle">
							<input type="checkbox" bind:checked={notifications.dailyReport} />
							<span class="toggle-switch"></span>
							<span class="toggle-label">Daily summary report email</span>
						</label>
					</div>

					<button class="btn btn-primary" on:click={saveNotifications} style="margin-top: 16px;">
						Save Preferences
					</button>
				</div>
			</div>

		<!-- System Info Tab -->
		{:else if activeTab === 'system'}
			<div class="section">
				<div class="section-header">
					<h3>System Information</h3>
					<span class="section-desc">SCADA system status and configuration reference</span>
				</div>

				<div class="info-grid">
					<div class="info-card">
						<h4>Connection Status</h4>
						<div class="info-row">
							<span class="info-label">WebSocket</span>
							<StatusIndicator status={$wsConnected ? 'connected' : 'disconnected'} size="md" />
						</div>
						<div class="info-row">
							<span class="info-label">API Endpoint</span>
							<span class="info-value mono">/api/v1</span>
						</div>
						<div class="info-row">
							<span class="info-label">WS Endpoint</span>
							<span class="info-value mono">/ws</span>
						</div>
					</div>

					<div class="info-card">
						<h4>Application</h4>
						<div class="info-row">
							<span class="info-label">Version</span>
							<span class="info-value mono">1.0.0</span>
						</div>
						<div class="info-row">
							<span class="info-label">Frontend</span>
							<span class="info-value mono">SvelteKit 2.x</span>
						</div>
						<div class="info-row">
							<span class="info-label">Backend</span>
							<span class="info-value mono">Go / Fiber</span>
						</div>
						<div class="info-row">
							<span class="info-label">Database</span>
							<span class="info-value mono">TimescaleDB</span>
						</div>
						<div class="info-row">
							<span class="info-label">Message Broker</span>
							<span class="info-value mono">MQTT / EMQX</span>
						</div>
					</div>
				</div>

				<!-- MQTT Topic Reference -->
				<div class="mqtt-ref">
					<h4>MQTT Topic Reference</h4>
					<div class="ref-table-wrap">
						<table class="ref-table">
							<thead>
								<tr><th>Topic Pattern</th><th>Direction</th><th>Description</th></tr>
							</thead>
							<tbody>
								<tr><td class="mono">scada/water/{'{device_id}'}/telemetry</td><td>Device &rarr; Server</td><td>Water sensor readings</td></tr>
								<tr><td class="mono">scada/water/{'{device_id}'}/status</td><td>Device &rarr; Server</td><td>Device status changes</td></tr>
								<tr><td class="mono">scada/water/{'{device_id}'}/command</td><td>Server &rarr; Device</td><td>Control commands (pump, valve)</td></tr>
								<tr><td class="mono">scada/solar/{'{device_id}'}/telemetry</td><td>Device &rarr; Server</td><td>Solar panel readings</td></tr>
								<tr><td class="mono">scada/solar/{'{device_id}'}/status</td><td>Device &rarr; Server</td><td>Inverter/panel status</td></tr>
								<tr><td class="mono">scada/solar/{'{device_id}'}/command</td><td>Server &rarr; Device</td><td>Inverter commands</td></tr>
								<tr><td class="mono">scada/system/heartbeat</td><td>Server</td><td>Backend heartbeat</td></tr>
								<tr><td class="mono">scada/alarms/{'{subsystem}'}</td><td>Server</td><td>Alarm notifications</td></tr>
							</tbody>
						</table>
					</div>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.settings-page { max-width: 1200px; margin: 0 auto; }

	.page-header { margin-bottom: 16px; }
	.page-header h2 { font-size: 1.3rem; display: flex; align-items: center; }

	/* Tabs */
	.tabs {
		display: flex;
		gap: 2px;
		margin-bottom: 20px;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 4px;
		overflow-x: auto;
	}
	.tab-btn {
		flex: 1;
		padding: 10px 16px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.8rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}
	.tab-btn:hover { background: var(--bg-card-hover); color: var(--text-primary); }
	.tab-btn.active { background: var(--color-info); color: white; font-weight: 600; }

	/* Section */
	.section {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 20px;
	}
	.section-header {
		margin-bottom: 16px;
		padding-bottom: 12px;
		border-bottom: 1px solid var(--border-color);
	}
	.section-header h3 { font-size: 1rem; font-weight: 600; margin-bottom: 4px; }
	.section-desc { font-size: 0.75rem; color: var(--text-muted); }

	.save-status {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
		border-radius: 6px;
		font-size: 0.8rem;
		margin-bottom: 14px;
	}
	.save-status.success {
		background: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.2);
		color: #22c55e;
	}

	/* Setpoint Table */
	.setpoint-table-wrap { overflow-x: auto; }
	.setpoint-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.setpoint-table th {
		padding: 10px 14px;
		text-align: left;
		font-size: 0.68rem;
		text-transform: uppercase;
		color: var(--text-muted);
		letter-spacing: 0.04em;
		border-bottom: 1px solid var(--border-color);
		background: var(--bg-secondary);
	}
	.setpoint-table td {
		padding: 10px 14px;
		border-bottom: 1px solid var(--border-color);
	}
	.setpoint-table tbody tr:hover { background: var(--bg-card-hover); }
	.sp-name { font-weight: 500; }
	.sp-value { font-family: var(--font-mono); font-weight: 700; color: var(--color-info); font-size: 0.9rem; }
	.sp-unit { font-size: 0.75rem; color: var(--text-muted); font-family: var(--font-mono); }
	.sp-input {
		background: var(--bg-secondary);
		color: var(--text-primary);
		border: 2px solid var(--color-info);
		padding: 4px 8px;
		border-radius: 4px;
		width: 90px;
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 700;
	}
	.sp-input:focus { outline: none; box-shadow: 0 0 0 2px rgba(59,130,246,0.2); }
	.sp-actions { white-space: nowrap; display: flex; gap: 4px; }
	.sp-btn {
		padding: 4px 10px;
		border-radius: 4px;
		font-size: 0.65rem;
		font-weight: 700;
		cursor: pointer;
		border: 1px solid;
		transition: all 0.15s;
		letter-spacing: 0.03em;
	}
	.sp-btn.edit { background: rgba(59,130,246,0.1); color: #3b82f6; border-color: rgba(59,130,246,0.2); }
	.sp-btn.edit:hover { background: rgba(59,130,246,0.2); }
	.sp-btn.save { background: rgba(34,197,94,0.1); color: #22c55e; border-color: rgba(34,197,94,0.2); }
	.sp-btn.save:hover { background: rgba(34,197,94,0.2); }
	.sp-btn.cancel { background: transparent; color: var(--text-muted); border-color: var(--border-color); }
	.sp-btn.cancel:hover { background: var(--bg-card-hover); }

	.subsystem-tag {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		padding: 2px 8px;
		border-radius: 3px;
	}
	.subsystem-tag.water { color: var(--color-water); background: rgba(6,182,212,0.1); }
	.subsystem-tag.solar { color: var(--color-solar); background: rgba(245,158,11,0.1); }

	/* Device List */
	.device-list { display: flex; flex-direction: column; gap: 8px; }
	.device-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 14px;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		transition: all 0.15s;
	}
	.device-card:hover { background: var(--bg-card-hover); }
	.device-card.fault { border-color: rgba(239,68,68,0.3); background: rgba(239,68,68,0.03); }
	.dc-main { display: flex; align-items: center; gap: 12px; }
	.dc-icon {
		width: 38px;
		height: 38px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.dc-icon.water { background: rgba(6,182,212,0.1); color: var(--color-water); }
	.dc-icon.solar { background: rgba(245,158,11,0.1); color: var(--color-solar); }
	.dc-name { font-size: 0.85rem; font-weight: 600; }
	.dc-meta { display: flex; gap: 8px; margin-top: 2px; }
	.dc-type { font-size: 0.68rem; color: var(--text-muted); }
	.dc-id { font-size: 0.62rem; color: var(--text-muted); font-family: var(--font-mono); }
	.dc-right { display: flex; align-items: center; gap: 12px; }
	.dc-lastseen { font-size: 0.68rem; color: var(--text-muted); font-family: var(--font-mono); white-space: nowrap; }

	/* Profile */
	.profile-card {
		display: flex;
		gap: 24px;
		align-items: flex-start;
	}
	.profile-avatar {
		width: 64px;
		height: 64px;
		border-radius: 16px;
		background: linear-gradient(135deg, var(--color-info), var(--color-water));
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.5rem;
		font-weight: 800;
		color: white;
		flex-shrink: 0;
	}
	.profile-details { flex: 1; }
	.pd-row {
		display: flex;
		align-items: center;
		padding: 10px 0;
		border-bottom: 1px solid var(--border-color);
		gap: 16px;
	}
	.pd-row:last-child { border-bottom: none; }
	.pd-label { font-size: 0.75rem; color: var(--text-muted); min-width: 120px; text-transform: uppercase; letter-spacing: 0.03em; }
	.pd-value { font-size: 0.85rem; color: var(--text-primary); }
	.role-badge {
		font-size: 0.65rem;
		font-weight: 700;
		padding: 3px 10px;
		border-radius: 4px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.role-badge.admin { background: rgba(139,92,246,0.15); color: #8b5cf6; }
	.role-badge.operator { background: rgba(59,130,246,0.15); color: #3b82f6; }
	.role-badge.viewer { background: rgba(107,114,128,0.15); color: #9ca3af; }

	/* Notifications */
	.notif-form { max-width: 500px; }
	.notif-group { margin-bottom: 20px; }
	.notif-group h4 { font-size: 0.8rem; color: var(--text-secondary); margin-bottom: 10px; }
	.notif-toggle {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 0;
		cursor: pointer;
		user-select: none;
	}
	.notif-toggle input { display: none; }
	.toggle-switch {
		width: 40px;
		height: 22px;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 11px;
		position: relative;
		transition: all 0.2s;
		flex-shrink: 0;
	}
	.toggle-switch::after {
		content: '';
		position: absolute;
		top: 2px;
		left: 2px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--text-muted);
		transition: all 0.2s;
	}
	.notif-toggle input:checked + .toggle-switch {
		background: rgba(59,130,246,0.2);
		border-color: var(--color-info);
	}
	.notif-toggle input:checked + .toggle-switch::after {
		left: 20px;
		background: var(--color-info);
	}
	.toggle-label { font-size: 0.8rem; color: var(--text-primary); }

	/* System Info */
	.info-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 16px;
		margin-bottom: 20px;
	}
	.info-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
	}
	.info-card h4 { font-size: 0.8rem; color: var(--text-secondary); margin-bottom: 12px; }
	.info-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 0;
		border-bottom: 1px solid var(--border-color);
	}
	.info-row:last-child { border-bottom: none; }
	.info-label { font-size: 0.75rem; color: var(--text-muted); }
	.info-value { font-size: 0.8rem; }
	.mono { font-family: var(--font-mono); font-size: 0.75rem; }

	.mqtt-ref {
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
	}
	.mqtt-ref h4 { font-size: 0.8rem; color: var(--text-secondary); margin-bottom: 12px; }
	.ref-table-wrap { overflow-x: auto; }
	.ref-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.ref-table th {
		text-align: left;
		padding: 8px 12px;
		font-size: 0.68rem;
		text-transform: uppercase;
		color: var(--text-muted);
		border-bottom: 1px solid var(--border-color);
	}
	.ref-table td { padding: 8px 12px; border-bottom: 1px solid var(--border-color); }
	.ref-table tbody tr:hover { background: var(--bg-card-hover); }

	/* Responsive */
	@media (max-width: 768px) {
		.tabs { overflow-x: auto; }
		.profile-card { flex-direction: column; }
		.info-grid { grid-template-columns: 1fr; }
		.pd-row { flex-direction: column; gap: 4px; align-items: flex-start; }
	}
</style>
