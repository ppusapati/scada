<script>
	import { onMount, onDestroy } from 'svelte';
	import { currentWaterSystem, loadWaterData } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { controlPump, controlValve, setWaterMode } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';
	import TankGauge from '$lib/components/water/TankGauge.svelte';

	let loading = false;
	let commandStatus = '';

	$: system = $currentWaterSystem;

	// Subscribe to real-time water updates
	let unsubscribe;
	onMount(() => {
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('telemetry', (msg) => {
			if (msg.subsystem === 'water') {
				loadWaterData();
			}
		});
	});

	onDestroy(() => {
		if (unsubscribe) unsubscribe();
	});

	async function handlePump(action) {
		if (!system) return;
		loading = true;
		commandStatus = `Sending ${action} command...`;
		try {
			await controlPump(system.id, action);
			commandStatus = `Pump ${action} command sent`;
			setTimeout(() => loadWaterData(), 1000);
		} catch (e) {
			commandStatus = `Error: ${e.message}`;
		} finally {
			loading = false;
		}
	}

	async function handleModeChange(e) {
		if (!system) return;
		const mode = e.target.value;
		try {
			await setWaterMode(system.id, mode);
			loadWaterData();
		} catch (e) {
			commandStatus = `Error: ${e.message}`;
		}
	}
</script>

<div class="water-page">
	<header class="page-header">
		<h2>Water Treatment System</h2>
		{#if system}
			<div class="header-controls">
				<span class="badge" class:badge-success={system.pump_status === 'running'}
					class:badge-warning={system.pump_status === 'stopped'}
					class:badge-critical={system.pump_status === 'fault'}>
					Pump: {system.pump_status}
				</span>
				<select class="mode-select" value={system.operating_mode} on:change={handleModeChange} disabled={!$isOperator}>
					<option value="auto">Auto</option>
					<option value="manual">Manual</option>
					<option value="maintenance">Maintenance</option>
				</select>
			</div>
		{/if}
	</header>

	{#if system}
		<!-- P&ID Style Layout -->
		<div class="pid-layout">
			<!-- Tank Visualization -->
			<div class="tank-section">
				<TankGauge level={system.tank_level} label="Main Tank" />
			</div>

			<!-- Process Metrics -->
			<div class="process-section">
				<div class="grid-3">
					<div class="process-card">
						<div class="process-label">Flow Rate In</div>
						<div class="process-value water-color">{system.flow_rate_in.toFixed(1)}</div>
						<div class="process-unit">L/min</div>
						<div class="process-bar">
							<div class="bar-fill" style="width: {Math.min(system.flow_rate_in / 100 * 100, 100)}%; background: var(--color-water);"></div>
						</div>
					</div>

					<div class="process-card">
						<div class="process-label">Flow Rate Out</div>
						<div class="process-value water-color">{system.flow_rate_out.toFixed(1)}</div>
						<div class="process-unit">L/min</div>
						<div class="process-bar">
							<div class="bar-fill" style="width: {Math.min(system.flow_rate_out / 100 * 100, 100)}%; background: var(--color-info);"></div>
						</div>
					</div>

					<div class="process-card">
						<div class="process-label">Pressure</div>
						<div class="process-value" class:danger={system.pressure > 6}>{system.pressure.toFixed(2)}</div>
						<div class="process-unit">bar</div>
						<div class="process-bar">
							<div class="bar-fill" style="width: {Math.min(system.pressure / 10 * 100, 100)}%; background: {system.pressure > 6 ? 'var(--color-danger)' : 'var(--color-success)'}"></div>
						</div>
					</div>

					<div class="process-card">
						<div class="process-label">pH Level</div>
						<div class="process-value" class:danger={system.ph_level < 6.5 || system.ph_level > 8.5}>
							{system.ph_level.toFixed(2)}
						</div>
						<div class="process-unit">pH</div>
						<div class="ph-indicator">
							<div class="ph-marker" style="left: {(system.ph_level / 14) * 100}%"></div>
						</div>
					</div>

					<div class="process-card">
						<div class="process-label">Turbidity</div>
						<div class="process-value" class:warning={system.turbidity > 2} class:danger={system.turbidity > 4}>
							{system.turbidity.toFixed(2)}
						</div>
						<div class="process-unit">NTU</div>
					</div>

					<div class="process-card">
						<div class="process-label">Chlorine</div>
						<div class="process-value" class:warning={system.chlorine_level < 0.2}>
							{system.chlorine_level.toFixed(2)}
						</div>
						<div class="process-unit">mg/L</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Control Panel -->
		{#if $isOperator}
			<div class="control-panel">
				<h3>Pump Control</h3>
				<div class="control-buttons">
					<button class="btn btn-success" on:click={() => handlePump('start')}
						disabled={loading || system.pump_status === 'running'}>
						Start Pump
					</button>
					<button class="btn btn-danger" on:click={() => handlePump('stop')}
						disabled={loading || system.pump_status === 'stopped'}>
						Stop Pump
					</button>
				</div>
				{#if commandStatus}
					<div class="command-feedback">{commandStatus}</div>
				{/if}
			</div>
		{/if}
	{:else}
		<div class="loading">Loading water system data...</div>
	{/if}
</div>

<style>
	.water-page { max-width: 1400px; margin: 0 auto; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
	}
	.page-header h2 { color: var(--color-water); }

	.header-controls { display: flex; align-items: center; gap: 12px; }

	.mode-select {
		background: var(--bg-secondary);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 6px 12px;
		border-radius: 6px;
		font-size: 0.85rem;
	}

	.pid-layout {
		display: grid;
		grid-template-columns: 200px 1fr;
		gap: 24px;
		margin-bottom: 24px;
	}

	.process-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		text-align: center;
	}
	.process-label { font-size: 0.7rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px; }
	.process-value { font-family: var(--font-mono); font-size: 1.8rem; font-weight: 700; color: var(--text-primary); }
	.process-value.water-color { color: var(--color-water); }
	.process-value.danger { color: var(--color-danger); }
	.process-value.warning { color: var(--color-warning); }
	.process-unit { font-size: 0.7rem; color: var(--text-muted); }

	.process-bar { height: 4px; background: var(--bg-secondary); border-radius: 2px; margin-top: 8px; overflow: hidden; }
	.bar-fill { height: 100%; border-radius: 2px; transition: width 0.5s ease; }

	.ph-indicator {
		height: 8px;
		background: linear-gradient(to right, #ef4444, #f59e0b, #22c55e, #3b82f6, #8b5cf6);
		border-radius: 4px;
		margin-top: 8px;
		position: relative;
	}
	.ph-marker {
		position: absolute;
		top: -3px;
		width: 4px;
		height: 14px;
		background: white;
		border-radius: 2px;
		transform: translateX(-50%);
	}

	.control-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 20px;
	}
	.control-panel h3 { margin-bottom: 12px; font-size: 0.9rem; color: var(--text-secondary); }
	.control-buttons { display: flex; gap: 12px; }
	.command-feedback { margin-top: 10px; font-size: 0.8rem; color: var(--text-muted); font-family: var(--font-mono); }

	.loading { text-align: center; padding: 60px; color: var(--text-muted); }
</style>
