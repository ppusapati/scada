<script>
	import { onMount, onDestroy } from 'svelte';
	import { currentSolarSystem, loadSolarData } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { controlInverter, setSolarMode } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';
	import StringTable from '$lib/components/solar/StringTable.svelte';
	import PowerGauge from '$lib/components/solar/PowerGauge.svelte';
	import EnergyChart from '$lib/components/solar/EnergyChart.svelte';
	import LineChart from '$lib/components/charts/LineChart.svelte';
	import GaugeChart from '$lib/components/charts/GaugeChart.svelte';

	let loading = false;
	let commandStatus = '';
	let commandTimeout;

	$: system = $currentSolarSystem;
	$: outputPercent = system ? (system.current_output_kw / system.total_capacity_kw * 100) : 0;

	// Trend data
	let powerTrend = [];
	let irradianceTrend = [];
	let tempTrend = [];
	let efficiencyTrend = [];
	let trendInterval;

	// Mock string data (simulate 16 strings)
	let stringData = [];

	function generateTrend(baseValue, variance, count) {
		const data = [];
		const now = Date.now();
		for (let i = count - 1; i >= 0; i--) {
			const timestamp = new Date(now - i * 5 * 60 * 1000).toISOString();
			const hourOfDay = new Date(now - i * 5 * 60 * 1000).getHours();
			// Solar curve: peaks at noon
			const solarFactor = Math.max(0, Math.sin((hourOfDay - 6) / 12 * Math.PI));
			const noise = (Math.random() - 0.5) * variance * 0.5;
			data.push({ timestamp, value: Math.max(0, baseValue * solarFactor + noise) });
		}
		return data;
	}

	function generateStringData() {
		const strings = [];
		for (let i = 1; i <= 16; i++) {
			const isFault = i === 12;
			const isWarn = i === 5;
			const baseV = 38 + Math.random() * 4;
			const baseI = 8 + Math.random() * 2;
			strings.push({
				id: i,
				name: `String ${i}`,
				voltage: isFault ? 0 : baseV,
				current: isFault ? 0 : baseI,
				power: isFault ? 0 : baseV * baseI,
				status: isFault ? 'fault' : isWarn ? 'warning' : 'normal',
				temperature: 38 + Math.random() * 15
			});
		}
		return strings;
	}

	function addPoint(data, value, max) {
		const updated = [...data, { timestamp: new Date().toISOString(), value }];
		if (updated.length > max) updated.shift();
		return updated;
	}

	let unsubscribe;
	onMount(() => {
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('telemetry', (msg) => {
			if (msg.subsystem === 'solar') loadSolarData();
		});

		// Initialize trends
		powerTrend = generateTrend(80, 20, 60);
		irradianceTrend = generateTrend(800, 100, 60);
		tempTrend = generateTrend(45, 5, 60);
		efficiencyTrend = generateTrend(19, 2, 60);
		stringData = generateStringData();

		trendInterval = setInterval(() => {
			if (system) {
				powerTrend = addPoint(powerTrend, system.current_output_kw, 60);
				irradianceTrend = addPoint(irradianceTrend, system.irradiance, 60);
				tempTrend = addPoint(tempTrend, system.panel_temperature, 60);
				efficiencyTrend = addPoint(efficiencyTrend, system.efficiency, 60);
			}
			// Slightly vary string data to simulate live updates
			stringData = stringData.map(s => ({
				...s,
				voltage: s.status === 'fault' ? 0 : s.voltage + (Math.random() - 0.5) * 0.2,
				current: s.status === 'fault' ? 0 : Math.max(0, s.current + (Math.random() - 0.5) * 0.1),
				power: s.status === 'fault' ? 0 : undefined,
				temperature: s.temperature + (Math.random() - 0.5) * 0.5
			})).map(s => ({ ...s, power: s.power ?? s.voltage * s.current }));
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

	async function handleInverter(action) {
		if (!system) return;
		loading = true;
		setStatus(`Sending inverter ${action} command...`);
		try {
			await controlInverter(system.id, action);
			setStatus(`Inverter ${action} command sent successfully`);
			setTimeout(() => loadSolarData(), 1000);
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
			await setSolarMode(system.id, mode);
			setStatus(`Mode changed to ${mode}`);
			loadSolarData();
		} catch (e) {
			setStatus(`Error: ${e.message}`);
		}
	}
</script>

<div class="solar-page">
	<header class="page-header">
		<div class="header-title">
			<h2>
				<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:inline-block; vertical-align:middle; margin-right:8px; color:var(--color-solar)">
					<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/>
				</svg>
				Solar Energy System
			</h2>
			{#if system}
				<span class="system-name">{system.name} ({system.id})</span>
			{/if}
		</div>
		{#if system}
			<div class="header-controls">
				<span class="badge" class:badge-success={system.inverter_status === 'online'}
					class:badge-warning={system.inverter_status === 'standby'}
					class:badge-critical={system.inverter_status === 'fault'}>
					Inverter: {system.inverter_status}
				</span>
				<div class="mode-selector">
					<label for="solar-mode">Mode:</label>
					<select id="solar-mode" class="mode-select" value={system.operating_mode} on:change={handleModeChange} disabled={!$isOperator}>
						<option value="auto">AUTO</option>
						<option value="manual">MANUAL</option>
						<option value="maintenance">MAINTENANCE</option>
					</select>
				</div>
			</div>
		{/if}
	</header>

	<!-- Command Feedback -->
	{#if commandStatus}
		<div class="command-bar">
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
			{commandStatus}
		</div>
	{/if}

	{#if system}
		<!-- Power & Energy Section -->
		<div class="power-section">
			<div class="power-main">
				<PowerGauge
					currentOutput={system.current_output_kw}
					totalCapacity={system.total_capacity_kw}
					dailyEnergy={system.daily_energy_kwh}
					totalEnergy={system.total_energy_mwh}
				/>
			</div>

			<!-- Key Metrics Sidebar -->
			<div class="solar-metrics-panel">
				<div class="sm-card">
					<div class="sm-label">Irradiance</div>
					<div class="sm-value">{system.irradiance.toFixed(0)}</div>
					<div class="sm-unit">W/m<sup>2</sup></div>
					<div class="sm-bar">
						<div class="sm-fill" style="width: {Math.min(system.irradiance / 1200 * 100, 100)}%; background: var(--color-solar)"></div>
					</div>
				</div>
				<div class="sm-card">
					<div class="sm-label">Panel Temperature</div>
					<div class="sm-value" class:hot={system.panel_temperature > 70} class:warn={system.panel_temperature > 55}>{system.panel_temperature.toFixed(1)}</div>
					<div class="sm-unit">&deg;C</div>
				</div>
				<div class="sm-card">
					<div class="sm-label">Grid Voltage</div>
					<div class="sm-value">{system.grid_voltage.toFixed(1)}</div>
					<div class="sm-unit">V</div>
				</div>
				<div class="sm-card">
					<div class="sm-label">Grid Frequency</div>
					<div class="sm-value">{system.grid_frequency.toFixed(2)}</div>
					<div class="sm-unit">Hz</div>
				</div>
				<div class="sm-card">
					<div class="sm-label">Efficiency</div>
					<div class="sm-value" style="color: {system.efficiency > 18 ? 'var(--color-success)' : 'var(--color-warning)'}">{system.efficiency.toFixed(1)}</div>
					<div class="sm-unit">%</div>
				</div>
				<div class="sm-card">
					<div class="sm-label">Capacity</div>
					<div class="sm-value solar-color">{system.total_capacity_kw.toFixed(0)}</div>
					<div class="sm-unit">kW</div>
				</div>
			</div>
		</div>

		<!-- Gauges Row -->
		<div class="gauges-row">
			<div class="gauge-card">
				<GaugeChart value={system.irradiance} min={0} max={1200} label="Irradiance" unit="W/m2" size={130} color="#f59e0b" />
			</div>
			<div class="gauge-card">
				<GaugeChart value={system.panel_temperature} min={0} max={100} label="Panel Temp" unit="C" size={130} color="#ef4444" warningThreshold={60} dangerThreshold={80} />
			</div>
			<div class="gauge-card">
				<GaugeChart value={system.efficiency} min={0} max={30} label="Efficiency" unit="%" size={130} color="#8b5cf6" />
			</div>
			<div class="gauge-card">
				<GaugeChart value={system.grid_frequency} min={49} max={51} label="Grid Freq" unit="Hz" size={130} color="#22c55e" warningThreshold={50.5} dangerThreshold={51} decimals={2} />
			</div>
		</div>

		<!-- String Performance Table -->
		<div class="string-section">
			<StringTable strings={stringData} />
		</div>

		<!-- Energy Production Chart -->
		<div class="energy-section">
			<EnergyChart height={220} />
		</div>

		<!-- Inverter Control -->
		{#if $isOperator}
			<div class="inverter-control">
				<h3 class="section-title">Inverter Control</h3>
				<div class="inverter-panel">
					<div class="inv-status">
						<div class="inv-detail">
							<span class="inv-label">Status</span>
							<span class="inv-value" class:inv-online={system.inverter_status === 'online'} class:inv-fault={system.inverter_status === 'fault'}>
								{system.inverter_status.toUpperCase()}
							</span>
						</div>
						<div class="inv-detail">
							<span class="inv-label">Output</span>
							<span class="inv-value solar-color">{system.current_output_kw.toFixed(1)} kW</span>
						</div>
						<div class="inv-detail">
							<span class="inv-label">Load</span>
							<span class="inv-value">{outputPercent.toFixed(1)}%</span>
						</div>
					</div>
					<div class="inv-actions">
						<button class="inv-btn enable" on:click={() => handleInverter('enable')} disabled={loading || system.inverter_status === 'online'}>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
							ENABLE
						</button>
						<button class="inv-btn disable" on:click={() => handleInverter('disable')} disabled={loading || system.inverter_status === 'offline'}>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2"/></svg>
							DISABLE
						</button>
						<button class="inv-btn reset" on:click={() => handleInverter('reset')} disabled={loading}>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
							RESET
						</button>
					</div>
				</div>
			</div>
		{/if}

		<!-- Trend Charts -->
		<div class="trends-section">
			<h3 class="section-title">Performance Trends (5-hour window)</h3>
			<div class="trends-grid">
				<div class="trend-card">
					<LineChart data={powerTrend} height={150} color="#f59e0b" label="Power Output" unit="kW" yMin={0} />
				</div>
				<div class="trend-card">
					<LineChart data={irradianceTrend} height={150} color="#fb923c" label="Solar Irradiance" unit="W/m2" yMin={0} />
				</div>
				<div class="trend-card">
					<LineChart data={tempTrend} height={150} color="#ef4444" label="Panel Temperature" unit="C" yMin={20} yMax={80} />
				</div>
				<div class="trend-card">
					<LineChart data={efficiencyTrend} height={150} color="#8b5cf6" label="System Efficiency" unit="%" yMin={0} yMax={25} />
				</div>
			</div>
		</div>
	{:else}
		<div class="loading-state">
			<div class="loading-spinner"></div>
			<p>Loading solar system data...</p>
			<p class="loading-hint">If this takes too long, the backend may be unavailable.</p>
		</div>
	{/if}
</div>

<style>
	.solar-page { max-width: 1400px; margin: 0 auto; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 20px;
		flex-wrap: wrap;
		gap: 12px;
	}
	.header-title h2 { color: var(--color-solar); font-size: 1.3rem; display: flex; align-items: center; }
	.system-name { font-size: 0.75rem; color: var(--text-muted); font-family: var(--font-mono); }
	.header-controls { display: flex; align-items: center; gap: 12px; }
	.mode-selector { display: flex; align-items: center; gap: 6px; }
	.mode-selector label { font-size: 0.75rem; color: var(--text-muted); text-transform: uppercase; }
	.mode-select { background: var(--bg-secondary); color: var(--text-primary); border: 1px solid var(--border-color); padding: 6px 12px; border-radius: 6px; font-size: 0.8rem; font-weight: 600; }

	.command-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		background: rgba(245, 158, 11, 0.1);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: 6px;
		margin-bottom: 16px;
		font-size: 0.8rem;
		color: var(--color-solar);
		font-family: var(--font-mono);
	}

	/* Power Section */
	.power-section {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin-bottom: 18px;
	}
	.power-main {
		display: flex;
	}
	.power-main :global(.power-gauge-panel) {
		flex: 1;
	}
	.solar-metrics-panel {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
	}
	.sm-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 12px;
		text-align: center;
	}
	.sm-card:hover { background: var(--bg-card-hover); }
	.sm-label { font-size: 0.6rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 4px; }
	.sm-value { font-family: var(--font-mono); font-size: 1.3rem; font-weight: 700; }
	.sm-value.solar-color { color: var(--color-solar); }
	.sm-value.hot { color: var(--color-danger); }
	.sm-value.warn { color: var(--color-warning); }
	.sm-unit { font-size: 0.6rem; color: var(--text-muted); }
	.sm-bar { height: 3px; background: var(--bg-secondary); border-radius: 2px; margin-top: 6px; overflow: hidden; }
	.sm-fill { height: 100%; border-radius: 2px; transition: width 0.5s; }

	/* Gauges Row */
	.gauges-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
		margin-bottom: 18px;
	}
	.gauge-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
		display: flex;
		justify-content: center;
	}

	/* String Table */
	.string-section {
		margin-bottom: 18px;
	}

	/* Energy Chart */
	.energy-section {
		margin-bottom: 18px;
	}

	/* Inverter Control */
	.inverter-control {
		margin-bottom: 18px;
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
	.inverter-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 18px;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 20px;
		flex-wrap: wrap;
	}
	.inv-status {
		display: flex;
		gap: 24px;
	}
	.inv-detail {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.inv-label { font-size: 0.65rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.03em; }
	.inv-value { font-family: var(--font-mono); font-size: 1rem; font-weight: 700; }
	.inv-value.inv-online { color: var(--color-success); }
	.inv-value.inv-fault { color: var(--color-danger); }

	.inv-actions {
		display: flex;
		gap: 8px;
	}
	.inv-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 16px;
		border-radius: 6px;
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		cursor: pointer;
		transition: all 0.15s;
		border: 1px solid;
	}
	.inv-btn.enable { background: rgba(34,197,94,0.12); color: #22c55e; border-color: rgba(34,197,94,0.25); }
	.inv-btn.enable:hover:not(:disabled) { background: rgba(34,197,94,0.25); }
	.inv-btn.disable { background: rgba(239,68,68,0.12); color: #ef4444; border-color: rgba(239,68,68,0.25); }
	.inv-btn.disable:hover:not(:disabled) { background: rgba(239,68,68,0.25); }
	.inv-btn.reset { background: rgba(59,130,246,0.12); color: #3b82f6; border-color: rgba(59,130,246,0.25); }
	.inv-btn.reset:hover:not(:disabled) { background: rgba(59,130,246,0.25); }
	.inv-btn:disabled { opacity: 0.35; cursor: not-allowed; }

	/* Trends */
	.trends-section { margin-bottom: 20px; }
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
	.loading-state { text-align: center; padding: 80px 20px; color: var(--text-muted); }
	.loading-spinner { width: 40px; height: 40px; border: 3px solid var(--border-color); border-top-color: var(--color-solar); border-radius: 50%; margin: 0 auto 16px; animation: spin 1s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }
	.loading-hint { font-size: 0.75rem; margin-top: 8px; }

	/* Responsive */
	@media (max-width: 1200px) {
		.power-section { grid-template-columns: 1fr; }
		.gauges-row { grid-template-columns: repeat(2, 1fr); }
	}
	@media (max-width: 768px) {
		.solar-metrics-panel { grid-template-columns: 1fr; }
		.gauges-row { grid-template-columns: 1fr; }
		.trends-grid { grid-template-columns: 1fr; }
		.inv-status { flex-direction: column; gap: 8px; }
		.inv-actions { flex-direction: column; }
		.page-header { flex-direction: column; }
	}
</style>
