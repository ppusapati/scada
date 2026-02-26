<script>
	import { onMount, onDestroy } from 'svelte';
	import { currentSolarSystem, loadSolarData } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { controlInverter, setSolarMode } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';

	let loading = false;
	let commandStatus = '';

	$: system = $currentSolarSystem;
	$: outputPercent = system ? (system.current_output_kw / system.total_capacity_kw * 100) : 0;

	let unsubscribe;
	onMount(() => {
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('telemetry', (msg) => {
			if (msg.subsystem === 'solar') loadSolarData();
		});
	});
	onDestroy(() => { if (unsubscribe) unsubscribe(); });

	async function handleInverter(action) {
		if (!system) return;
		loading = true;
		commandStatus = `Sending ${action} command...`;
		try {
			await controlInverter(system.id, action);
			commandStatus = `Inverter ${action} command sent`;
			setTimeout(() => loadSolarData(), 1000);
		} catch (e) {
			commandStatus = `Error: ${e.message}`;
		} finally {
			loading = false;
		}
	}

	async function handleModeChange(e) {
		if (!system) return;
		try {
			await setSolarMode(system.id, e.target.value);
			loadSolarData();
		} catch (e) {
			commandStatus = `Error: ${e.message}`;
		}
	}
</script>

<div class="solar-page">
	<header class="page-header">
		<h2>Solar Energy System</h2>
		{#if system}
			<div class="header-controls">
				<span class="badge" class:badge-success={system.inverter_status === 'online'}
					class:badge-critical={system.inverter_status === 'fault'}>
					Inverter: {system.inverter_status}
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
		<!-- Power Output Visualization -->
		<div class="power-display card">
			<div class="power-header">
				<span class="power-label">Current Output</span>
				<span class="power-capacity">/ {system.total_capacity_kw} kW capacity</span>
			</div>
			<div class="power-value">{system.current_output_kw.toFixed(1)} <span class="power-unit">kW</span></div>
			<div class="power-bar-container">
				<div class="power-bar" style="width: {outputPercent}%"></div>
			</div>
			<div class="power-percentage">{outputPercent.toFixed(1)}% of capacity</div>
		</div>

		<!-- Solar Metrics Grid -->
		<div class="grid-4" style="margin-top: 20px;">
			<div class="solar-metric card">
				<div class="metric-label">Irradiance</div>
				<div class="metric-value solar-accent">{system.irradiance.toFixed(0)}</div>
				<div class="metric-unit">W/m²</div>
				<div class="irradiance-bar">
					<div class="irr-fill" style="width: {Math.min(system.irradiance / 1200 * 100, 100)}%"></div>
				</div>
			</div>

			<div class="solar-metric card">
				<div class="metric-label">Panel Temperature</div>
				<div class="metric-value" class:danger={system.panel_temperature > 80}>
					{system.panel_temperature.toFixed(1)}
				</div>
				<div class="metric-unit">°C</div>
			</div>

			<div class="solar-metric card">
				<div class="metric-label">Grid Voltage</div>
				<div class="metric-value">{system.grid_voltage.toFixed(1)}</div>
				<div class="metric-unit">V</div>
			</div>

			<div class="solar-metric card">
				<div class="metric-label">Grid Frequency</div>
				<div class="metric-value">{system.grid_frequency.toFixed(2)}</div>
				<div class="metric-unit">Hz</div>
			</div>
		</div>

		<!-- Energy Production -->
		<div class="grid-3" style="margin-top: 20px;">
			<div class="energy-card card">
				<div class="metric-label">Today's Production</div>
				<div class="metric-value solar-accent">{system.daily_energy_kwh.toFixed(1)}</div>
				<div class="metric-unit">kWh</div>
			</div>

			<div class="energy-card card">
				<div class="metric-label">Total Production</div>
				<div class="metric-value solar-accent">{system.total_energy_mwh.toFixed(2)}</div>
				<div class="metric-unit">MWh</div>
			</div>

			<div class="energy-card card">
				<div class="metric-label">Efficiency</div>
				<div class="metric-value" style="color: {system.efficiency > 18 ? 'var(--color-success)' : 'var(--color-warning)'}">
					{system.efficiency.toFixed(1)}
				</div>
				<div class="metric-unit">%</div>
			</div>
		</div>

		<!-- Inverter Controls -->
		{#if $isOperator}
			<div class="control-panel card" style="margin-top: 20px;">
				<h3>Inverter Control</h3>
				<div class="control-buttons">
					<button class="btn btn-success" on:click={() => handleInverter('enable')} disabled={loading}>
						Enable
					</button>
					<button class="btn btn-danger" on:click={() => handleInverter('disable')} disabled={loading}>
						Disable
					</button>
					<button class="btn" on:click={() => handleInverter('reset')} disabled={loading}>
						Reset
					</button>
				</div>
				{#if commandStatus}
					<div class="command-feedback">{commandStatus}</div>
				{/if}
			</div>
		{/if}
	{:else}
		<div class="loading">Loading solar system data...</div>
	{/if}
</div>

<style>
	.solar-page { max-width: 1400px; margin: 0 auto; }
	.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
	.page-header h2 { color: var(--color-solar); }
	.header-controls { display: flex; align-items: center; gap: 12px; }
	.mode-select { background: var(--bg-secondary); color: var(--text-primary); border: 1px solid var(--border-color); padding: 6px 12px; border-radius: 6px; }

	.power-display { text-align: center; padding: 32px; }
	.power-header { display: flex; justify-content: center; gap: 8px; align-items: baseline; margin-bottom: 8px; }
	.power-label { font-size: 0.8rem; color: var(--text-secondary); text-transform: uppercase; }
	.power-capacity { font-size: 0.75rem; color: var(--text-muted); }
	.power-value { font-family: var(--font-mono); font-size: 3.5rem; font-weight: 800; color: var(--color-solar); }
	.power-unit { font-size: 1.5rem; color: var(--text-muted); }
	.power-bar-container { height: 8px; background: var(--bg-secondary); border-radius: 4px; margin: 16px 0 8px; overflow: hidden; }
	.power-bar { height: 100%; background: linear-gradient(90deg, var(--color-solar), #fcd34d); border-radius: 4px; transition: width 0.5s; }
	.power-percentage { font-size: 0.8rem; color: var(--text-muted); }

	.solar-metric { text-align: center; padding: 20px; }
	.solar-accent { color: var(--color-solar) !important; }
	.danger { color: var(--color-danger) !important; }

	.irradiance-bar { height: 4px; background: var(--bg-secondary); border-radius: 2px; margin-top: 8px; overflow: hidden; }
	.irr-fill { height: 100%; background: var(--color-solar); border-radius: 2px; transition: width 0.5s; }

	.energy-card { text-align: center; padding: 24px; }

	.control-panel h3 { margin-bottom: 12px; font-size: 0.9rem; color: var(--text-secondary); }
	.control-buttons { display: flex; gap: 12px; }
	.command-feedback { margin-top: 10px; font-size: 0.8rem; color: var(--text-muted); font-family: var(--font-mono); }
	.loading { text-align: center; padding: 60px; color: var(--text-muted); }
</style>
