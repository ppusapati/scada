<script>
	import { onMount, onDestroy } from 'svelte';
	import { currentWaterSystem, loadWaterData } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { controlPump, controlValve, setWaterMode } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';
	import TankGauge from '$lib/components/water/TankGauge.svelte';
	import PumpControl from '$lib/components/water/PumpControl.svelte';
	import ValveControl from '$lib/components/water/ValveControl.svelte';
	import LineChart from '$lib/components/charts/LineChart.svelte';
	import GaugeChart from '$lib/components/charts/GaugeChart.svelte';

	let loading = false;
	let commandStatus = '';
	let commandTimeout;

	$: system = $currentWaterSystem;

	// Trend data
	let tankTrend = [];
	let flowTrend = [];
	let pressureTrend = [];
	let phTrend = [];
	let trendInterval;

	// Valve positions derived from system data
	$: valves = system ? [
		{ id: 'V-001', name: 'Inlet Valve', position: system.valve_positions?.['V-001'] ?? 100 },
		{ id: 'V-002', name: 'Outlet Valve', position: system.valve_positions?.['V-002'] ?? 75 },
		{ id: 'V-003', name: 'Bypass Valve', position: system.valve_positions?.['V-003'] ?? 0 },
		{ id: 'V-004', name: 'Drain Valve', position: system.valve_positions?.['V-004'] ?? 0 },
	] : [];

	function generateTrend(baseValue, variance, count) {
		const data = [];
		const now = Date.now();
		for (let i = count - 1; i >= 0; i--) {
			const timestamp = new Date(now - i * 5 * 60 * 1000).toISOString();
			const noise = (Math.random() - 0.5) * variance * 2;
			data.push({ timestamp, value: Math.max(0, baseValue + noise) });
		}
		return data;
	}

	function addPoint(data, value, max) {
		const updated = [...data, { timestamp: new Date().toISOString(), value }];
		if (updated.length > max) updated.shift();
		return updated;
	}

	// Subscribe to real-time water updates
	let unsubscribe;
	onMount(() => {
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('telemetry', (msg) => {
			if (msg.subsystem === 'water') {
				loadWaterData();
			}
		});

		// Initialize trends
		tankTrend = generateTrend(65, 5, 60);
		flowTrend = generateTrend(45, 10, 60);
		pressureTrend = generateTrend(3.5, 0.5, 60);
		phTrend = generateTrend(7.2, 0.3, 60);

		trendInterval = setInterval(() => {
			if (system) {
				tankTrend = addPoint(tankTrend, system.tank_level, 60);
				flowTrend = addPoint(flowTrend, system.flow_rate_in, 60);
				pressureTrend = addPoint(pressureTrend, system.pressure, 60);
				phTrend = addPoint(phTrend, system.ph_level, 60);
			}
		}, 30000);
	});

	onDestroy(() => {
		if (unsubscribe) unsubscribe();
		if (trendInterval) clearInterval(trendInterval);
		if (commandTimeout) clearTimeout(commandTimeout);
	});

	function setStatus(msg) {
		commandStatus = msg;
		if (commandTimeout) clearTimeout(commandTimeout);
		commandTimeout = setTimeout(() => { commandStatus = ''; }, 5000);
	}

	async function handlePump(e) {
		if (!system) return;
		const action = e.detail.action;
		loading = true;
		setStatus(`Sending ${action} command...`);
		try {
			await controlPump(system.id, action);
			setStatus(`Pump ${action} command sent successfully`);
			setTimeout(() => loadWaterData(), 1000);
		} catch (e) {
			setStatus(`Error: ${e.message}`);
		} finally {
			loading = false;
		}
	}

	async function handleValve(e) {
		if (!system) return;
		const { valve_id, position } = e.detail;
		loading = true;
		setStatus(`Setting ${valve_id} to ${position}%...`);
		try {
			await controlValve(system.id, valve_id, position);
			setStatus(`Valve ${valve_id} command sent`);
			setTimeout(() => loadWaterData(), 1000);
		} catch (e) {
			setStatus(`Error: ${e.message}`);
		} finally {
			loading = false;
		}
	}

	async function handleModeChange(e) {
		if (!system) return;
		const mode = e.target.value;
		setStatus(`Changing mode to ${mode}...`);
		try {
			await setWaterMode(system.id, mode);
			setStatus(`Mode changed to ${mode}`);
			loadWaterData();
		} catch (e) {
			setStatus(`Error: ${e.message}`);
		}
	}
</script>

<div class="water-page">
	<header class="page-header">
		<div class="header-title">
			<h2>
				<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:inline-block; vertical-align:middle; margin-right:8px; color:var(--color-water)">
					<path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/>
				</svg>
				Water Treatment System
			</h2>
			{#if system}
				<span class="system-name">{system.name} ({system.id})</span>
			{/if}
		</div>
		{#if system}
			<div class="header-controls">
				<span class="badge" class:badge-success={system.pump_status === 'running'}
					class:badge-warning={system.pump_status === 'stopped'}
					class:badge-critical={system.pump_status === 'fault'}>
					Pump: {system.pump_status}
				</span>
				<div class="mode-selector">
					<label for="water-mode">Mode:</label>
					<select id="water-mode" class="mode-select" value={system.operating_mode} on:change={handleModeChange} disabled={!$isOperator}>
						<option value="auto">AUTO</option>
						<option value="manual">MANUAL</option>
						<option value="maintenance">MAINTENANCE</option>
					</select>
				</div>
			</div>
		{/if}
	</header>

	<!-- Command feedback bar -->
	{#if commandStatus}
		<div class="command-bar">
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
			{commandStatus}
		</div>
	{/if}

	{#if system}
		<!-- Top Section: Tank + Process Values -->
		<div class="top-section">
			<!-- Tank Visualization -->
			<div class="tank-panel">
				<TankGauge level={system.tank_level} label="Main Storage Tank" />
				<div class="tank-info">
					<div class="ti-row">
						<span class="ti-label">Capacity</span>
						<span class="ti-value">50,000 L</span>
					</div>
					<div class="ti-row">
						<span class="ti-label">Current Volume</span>
						<span class="ti-value" style="color:var(--color-water)">{(system.tank_level / 100 * 50000).toFixed(0)} L</span>
					</div>
					<div class="ti-row">
						<span class="ti-label">Net Flow</span>
						<span class="ti-value" class:positive={system.flow_rate_in > system.flow_rate_out} class:negative={system.flow_rate_in < system.flow_rate_out}>
							{system.flow_rate_in > system.flow_rate_out ? '+' : ''}{(system.flow_rate_in - system.flow_rate_out).toFixed(1)} L/min
						</span>
					</div>
				</div>
			</div>

			<!-- Process Metrics -->
			<div class="process-metrics">
				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">Flow Rate In</span>
						<span class="pc-icon water">&#8594;</span>
					</div>
					<div class="pc-value water-color">{system.flow_rate_in.toFixed(1)}</div>
					<div class="pc-unit">L/min</div>
					<div class="pc-bar">
						<div class="pc-bar-fill" style="width: {Math.min(system.flow_rate_in / 100 * 100, 100)}%; background: var(--color-water);"></div>
					</div>
				</div>

				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">Flow Rate Out</span>
						<span class="pc-icon">&#8592;</span>
					</div>
					<div class="pc-value" style="color:var(--color-info)">{system.flow_rate_out.toFixed(1)}</div>
					<div class="pc-unit">L/min</div>
					<div class="pc-bar">
						<div class="pc-bar-fill" style="width: {Math.min(system.flow_rate_out / 100 * 100, 100)}%; background: var(--color-info);"></div>
					</div>
				</div>

				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">Pressure</span>
						<span class="pc-icon" class:danger-icon={system.pressure > 6}>P</span>
					</div>
					<div class="pc-value" class:danger={system.pressure > 6} class:warning={system.pressure > 5}>{system.pressure.toFixed(2)}</div>
					<div class="pc-unit">bar</div>
					<div class="pc-bar">
						<div class="pc-bar-fill" style="width: {Math.min(system.pressure / 10 * 100, 100)}%; background: {system.pressure > 6 ? 'var(--color-danger)' : system.pressure > 5 ? 'var(--color-warning)' : 'var(--color-success)'}"></div>
					</div>
				</div>

				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">pH Level</span>
						<span class="pc-icon">pH</span>
					</div>
					<div class="pc-value" class:danger={system.ph_level < 6.5 || system.ph_level > 8.5} class:warning={system.ph_level < 6.8 || system.ph_level > 8.2}>
						{system.ph_level.toFixed(2)}
					</div>
					<div class="pc-unit">pH</div>
					<div class="ph-indicator">
						<div class="ph-marker" style="left: {(system.ph_level / 14) * 100}%"></div>
						<div class="ph-range" style="left: {(6.5/14)*100}%; width: {((8.5-6.5)/14)*100}%"></div>
					</div>
					<div class="ph-labels">
						<span>0</span><span>7</span><span>14</span>
					</div>
				</div>

				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">Turbidity</span>
						<span class="pc-icon">NTU</span>
					</div>
					<div class="pc-value" class:warning={system.turbidity > 2} class:danger={system.turbidity > 4}>{system.turbidity.toFixed(2)}</div>
					<div class="pc-unit">NTU</div>
					<div class="pc-bar">
						<div class="pc-bar-fill" style="width: {Math.min(system.turbidity / 5 * 100, 100)}%; background: {system.turbidity > 4 ? 'var(--color-danger)' : system.turbidity > 2 ? 'var(--color-warning)' : 'var(--color-success)'}"></div>
					</div>
				</div>

				<div class="process-card">
					<div class="pc-header">
						<span class="pc-label">Chlorine</span>
						<span class="pc-icon">Cl</span>
					</div>
					<div class="pc-value" class:warning={system.chlorine_level < 0.2 || system.chlorine_level > 4}>{system.chlorine_level.toFixed(2)}</div>
					<div class="pc-unit">mg/L</div>
					<div class="pc-bar">
						<div class="pc-bar-fill" style="width: {Math.min(system.chlorine_level / 5 * 100, 100)}%; background: {system.chlorine_level < 0.2 ? 'var(--color-warning)' : 'var(--color-success)'}"></div>
					</div>
				</div>
			</div>
		</div>

		<!-- Gauges Row -->
		<div class="gauges-row">
			<div class="gauge-panel">
				<GaugeChart value={system.pressure} min={0} max={10} label="Pressure" unit="bar" size={130} color="#22c55e" warningThreshold={6} dangerThreshold={8} />
			</div>
			<div class="gauge-panel">
				<GaugeChart value={system.ph_level} min={0} max={14} label="pH Level" unit="pH" size={130} color="#3b82f6" warningThreshold={8.5} dangerThreshold={9.5} />
			</div>
			<div class="gauge-panel">
				<GaugeChart value={system.turbidity} min={0} max={10} label="Turbidity" unit="NTU" size={130} color="#8b5cf6" warningThreshold={3} dangerThreshold={5} />
			</div>
			<div class="gauge-panel">
				<GaugeChart value={system.chlorine_level} min={0} max={5} label="Chlorine" unit="mg/L" size={130} color="#06b6d4" warningThreshold={4} dangerThreshold={4.5} />
			</div>
		</div>

		<!-- Controls Section -->
		{#if $isOperator}
			<div class="controls-section">
				<h3 class="section-title">Equipment Controls</h3>
				<div class="controls-grid">
					<!-- Pump Controls -->
					<div class="controls-column">
						<h4 class="col-title">Pumps</h4>
						<PumpControl
							pumpId="P-001"
							pumpName="Main Feed Pump"
							status={system.pump_status}
							flowRate={system.flow_rate_in}
							runtime={43200}
							disabled={loading}
							on:command={handlePump}
						/>
					</div>

					<!-- Valve Controls -->
					<div class="controls-column wide">
						<h4 class="col-title">Valves</h4>
						<div class="valves-grid">
							{#each valves as valve}
								<ValveControl
									valveId={valve.id}
									valveName={valve.name}
									position={valve.position}
									disabled={loading || !$isOperator}
									on:command={handleValve}
								/>
							{/each}
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Historical Trends -->
		<div class="trends-section">
			<h3 class="section-title">Historical Trends (5-hour window)</h3>
			<div class="trends-grid">
				<div class="trend-card">
					<LineChart data={tankTrend} height={150} color="#06b6d4" label="Tank Level" unit="%" yMin={0} yMax={100} />
				</div>
				<div class="trend-card">
					<LineChart data={flowTrend} height={150} color="#3b82f6" label="Flow Rate In" unit="L/min" yMin={0} />
				</div>
				<div class="trend-card">
					<LineChart data={pressureTrend} height={150} color="#22c55e" label="Pressure" unit="bar" yMin={0} yMax={10} />
				</div>
				<div class="trend-card">
					<LineChart data={phTrend} height={150} color="#8b5cf6" label="pH Level" unit="pH" yMin={5} yMax={9} />
				</div>
			</div>
		</div>
	{:else}
		<div class="loading-state">
			<div class="loading-spinner"></div>
			<p>Loading water system data...</p>
			<p class="loading-hint">If this takes too long, the backend may be unavailable.</p>
		</div>
	{/if}
</div>

<style>
	.water-page { max-width: 1400px; margin: 0 auto; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 20px;
		flex-wrap: wrap;
		gap: 12px;
	}
	.header-title h2 {
		color: var(--color-water);
		font-size: 1.3rem;
		display: flex;
		align-items: center;
	}
	.system-name {
		font-size: 0.75rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.header-controls {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.mode-selector {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.mode-selector label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
	}
	.mode-select {
		background: var(--bg-secondary);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 6px 12px;
		border-radius: 6px;
		font-size: 0.8rem;
		font-weight: 600;
	}

	.command-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		background: rgba(59, 130, 246, 0.1);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: 6px;
		margin-bottom: 16px;
		font-size: 0.8rem;
		color: var(--color-info);
		font-family: var(--font-mono);
	}

	/* Top Section: Tank + Process */
	.top-section {
		display: grid;
		grid-template-columns: 220px 1fr;
		gap: 20px;
		margin-bottom: 20px;
	}
	.tank-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
	}
	.tank-info {
		width: 100%;
	}
	.ti-row {
		display: flex;
		justify-content: space-between;
		padding: 5px 0;
		border-bottom: 1px solid var(--border-color);
	}
	.ti-row:last-child { border-bottom: none; }
	.ti-label { font-size: 0.7rem; color: var(--text-muted); }
	.ti-value { font-size: 0.75rem; font-family: var(--font-mono); font-weight: 600; }
	.ti-value.positive { color: var(--color-success); }
	.ti-value.negative { color: var(--color-danger); }

	.process-metrics {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
	}
	.process-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
		text-align: center;
		transition: all 0.15s;
	}
	.process-card:hover {
		background: var(--bg-card-hover);
	}
	.pc-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}
	.pc-label { font-size: 0.68rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
	.pc-icon { font-size: 0.7rem; color: var(--text-muted); font-weight: 700; font-family: var(--font-mono); }
	.pc-icon.water { color: var(--color-water); }
	.pc-icon.danger-icon { color: var(--color-danger); }
	.pc-value { font-family: var(--font-mono); font-size: 1.8rem; font-weight: 700; color: var(--text-primary); line-height: 1.1; }
	.pc-value.water-color { color: var(--color-water); }
	.pc-value.danger { color: var(--color-danger); }
	.pc-value.warning { color: var(--color-warning); }
	.pc-unit { font-size: 0.65rem; color: var(--text-muted); margin-top: 2px; }

	.pc-bar { height: 4px; background: var(--bg-secondary); border-radius: 2px; margin-top: 8px; overflow: hidden; }
	.pc-bar-fill { height: 100%; border-radius: 2px; transition: width 0.5s ease; }

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
		box-shadow: 0 0 4px rgba(0,0,0,0.3);
		z-index: 2;
	}
	.ph-range {
		position: absolute;
		top: 0;
		height: 100%;
		border: 1.5px solid rgba(255,255,255,0.4);
		border-radius: 4px;
		z-index: 1;
	}
	.ph-labels {
		display: flex;
		justify-content: space-between;
		font-size: 0.55rem;
		color: var(--text-muted);
		margin-top: 2px;
		font-family: var(--font-mono);
	}

	/* Gauges */
	.gauges-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
		margin-bottom: 20px;
	}
	.gauge-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
		display: flex;
		justify-content: center;
	}

	/* Controls */
	.controls-section {
		margin-bottom: 20px;
	}
	.section-title {
		font-size: 0.85rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 12px;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--border-color);
	}
	.controls-grid {
		display: grid;
		grid-template-columns: 1fr 2fr;
		gap: 16px;
	}
	.col-title {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 8px;
	}
	.valves-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
	}

	/* Trends */
	.trends-section {
		margin-bottom: 20px;
	}
	.trends-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 14px;
	}
	.trend-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
	}

	/* Loading */
	.loading-state {
		text-align: center;
		padding: 80px 20px;
		color: var(--text-muted);
	}
	.loading-spinner {
		width: 40px;
		height: 40px;
		border: 3px solid var(--border-color);
		border-top-color: var(--color-water);
		border-radius: 50%;
		margin: 0 auto 16px;
		animation: spin 1s linear infinite;
	}
	@keyframes spin { to { transform: rotate(360deg); } }
	.loading-hint {
		font-size: 0.75rem;
		margin-top: 8px;
		color: var(--text-muted);
	}

	/* Responsive */
	@media (max-width: 1200px) {
		.top-section { grid-template-columns: 1fr; }
		.process-metrics { grid-template-columns: repeat(2, 1fr); }
		.controls-grid { grid-template-columns: 1fr; }
		.gauges-row { grid-template-columns: repeat(2, 1fr); }
	}
	@media (max-width: 768px) {
		.process-metrics,
		.valves-grid,
		.trends-grid,
		.gauges-row {
			grid-template-columns: 1fr;
		}
		.page-header {
			flex-direction: column;
		}
	}
</style>
