<script>
	import { onMount, onDestroy } from 'svelte';
	import { waterSystems, solarSystems, activeAlarms, alarmCounts } from '$lib/stores/scada';
	import { wsConnected, getWebSocket } from '$lib/services/websocket';
	import { auth } from '$lib/stores/auth';
	import MetricCard from '$lib/components/dashboard/MetricCard.svelte';
	import AlarmTicker from '$lib/components/dashboard/AlarmTicker.svelte';
	import SystemOverview from '$lib/components/dashboard/SystemOverview.svelte';
	import LineChart from '$lib/components/charts/LineChart.svelte';
	import GaugeChart from '$lib/components/charts/GaugeChart.svelte';

	$: water = $waterSystems?.[0] || null;
	$: solar = $solarSystems?.[0] || null;
	$: totalAlarms = ($alarmCounts?.critical || 0) + ($alarmCounts?.warning || 0) + ($alarmCounts?.info || 0);

	let now = new Date();
	let clockInterval;
	let waterTrendData = [];
	let solarTrendData = [];
	let trendInterval;

	// Generate initial trend data representing last 24 hours
	function generateTrendData(baseValue, variance, count) {
		const data = [];
		const now = Date.now();
		for (let i = count - 1; i >= 0; i--) {
			const timestamp = new Date(now - i * 5 * 60 * 1000).toISOString();
			const noise = (Math.random() - 0.5) * variance * 2;
			const timeFactor = Math.sin((count - i) / count * Math.PI * 2) * variance * 0.3;
			data.push({ timestamp, value: Math.max(0, baseValue + noise + timeFactor) });
		}
		return data;
	}

	function addTrendPoint(data, currentValue, maxPoints) {
		const updated = [...data, { timestamp: new Date().toISOString(), value: currentValue }];
		if (updated.length > maxPoints) updated.shift();
		return updated;
	}

	onMount(() => {
		clockInterval = setInterval(() => { now = new Date(); }, 1000);

		// Initialize trend data
		waterTrendData = generateTrendData(65, 8, 48);
		solarTrendData = generateTrendData(75, 15, 48);

		// Update trend data periodically
		trendInterval = setInterval(() => {
			if (water) {
				waterTrendData = addTrendPoint(waterTrendData, water.tank_level, 48);
			}
			if (solar) {
				solarTrendData = addTrendPoint(solarTrendData, solar.current_output_kw, 48);
			}
		}, 30000);
	});

	onDestroy(() => {
		if (clockInterval) clearInterval(clockInterval);
		if (trendInterval) clearInterval(trendInterval);
	});

	// Quick actions
	let quickActionFeedback = '';
	function showFeedback(msg) {
		quickActionFeedback = msg;
		setTimeout(() => { quickActionFeedback = ''; }, 3000);
	}
</script>

<div class="dashboard">
	<!-- Header Bar -->
	<header class="dash-header">
		<div>
			<h2>System Overview</h2>
			<span class="dash-date">{now.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}</span>
		</div>
		<div class="header-right">
			<span class="clock">{now.toLocaleTimeString()}</span>
		</div>
	</header>

	<!-- Alarm Ticker -->
	<AlarmTicker alarms={$activeAlarms || []} />

	<!-- KPI Cards Row -->
	<div class="kpi-grid">
		<MetricCard
			label="Water Tank Level"
			value={water ? water.tank_level.toFixed(1) : '--'}
			unit="%"
			color="var(--color-water)"
			icon="water"
			trend={water ? (water.tank_level < 20 ? 'critical' : water.tank_level < 40 ? 'low' : 'normal') : 'normal'}
		/>
		<MetricCard
			label="Solar Output"
			value={solar ? solar.current_output_kw.toFixed(1) : '--'}
			unit="kW"
			color="var(--color-solar)"
			icon="solar"
			trend="normal"
		/>
		<MetricCard
			label="Active Alarms"
			value={totalAlarms}
			unit=""
			color={($alarmCounts?.critical || 0) > 0 ? 'var(--color-danger)' : 'var(--color-success)'}
			icon="alarm"
			trend={($alarmCounts?.critical || 0) > 0 ? 'critical' : 'normal'}
		/>
		<MetricCard
			label="System Efficiency"
			value={solar ? solar.efficiency.toFixed(1) : '--'}
			unit="%"
			color="var(--color-power)"
			icon="power"
			trend="normal"
		/>
	</div>

	<!-- System Panels + Gauges Row -->
	<div class="overview-grid">
		<!-- Water System Panel -->
		<SystemOverview
			title="Water Treatment Plant"
			subsystem="water"
			metrics={water ? [
				{ label: 'Flow In', value: water.flow_rate_in, unit: 'L/min' },
				{ label: 'Flow Out', value: water.flow_rate_out, unit: 'L/min' },
				{ label: 'Pressure', value: water.pressure, unit: 'bar' },
				{ label: 'pH Level', value: water.ph_level, unit: 'pH' },
				{ label: 'Turbidity', value: water.turbidity, unit: 'NTU' },
				{ label: 'Chlorine', value: water.chlorine_level, unit: 'mg/L' },
			] : []}
			status={water?.pump_status ?? 'unknown'}
			mode={water?.operating_mode ?? 'unknown'}
		/>

		<!-- Solar System Panel -->
		<SystemOverview
			title="Solar Array Alpha"
			subsystem="solar"
			metrics={solar ? [
				{ label: 'Capacity', value: solar.total_capacity_kw, unit: 'kW' },
				{ label: 'Output', value: solar.current_output_kw, unit: 'kW' },
				{ label: 'Irradiance', value: solar.irradiance, unit: 'W/m2' },
				{ label: 'Panel Temp', value: solar.panel_temperature, unit: 'C' },
				{ label: 'Daily Energy', value: solar.daily_energy_kwh, unit: 'kWh' },
				{ label: 'Efficiency', value: solar.efficiency, unit: '%' },
			] : []}
			status={solar?.inverter_status ?? 'unknown'}
			mode={solar?.operating_mode ?? 'unknown'}
		/>
	</div>

	<!-- Mini Trend Charts -->
	<div class="trend-section">
		<div class="trend-card">
			<LineChart
				data={waterTrendData}
				height={160}
				color="#06b6d4"
				label="Tank Level - Last 24h"
				unit="%"
				yMin={0}
				yMax={100}
			/>
		</div>
		<div class="trend-card">
			<LineChart
				data={solarTrendData}
				height={160}
				color="#f59e0b"
				label="Solar Output - Last 24h"
				unit="kW"
				yMin={0}
			/>
		</div>
	</div>

	<!-- Gauges Row -->
	<div class="gauges-row">
		<div class="gauge-card">
			<GaugeChart
				value={water ? water.tank_level : 0}
				min={0}
				max={100}
				label="Tank Level"
				unit="%"
				size={140}
				color="#06b6d4"
				warningThreshold={30}
				dangerThreshold={15}
			/>
		</div>
		<div class="gauge-card">
			<GaugeChart
				value={solar ? solar.current_output_kw : 0}
				min={0}
				max={solar ? solar.total_capacity_kw : 100}
				label="Solar Power"
				unit="kW"
				size={140}
				color="#f59e0b"
				warningThreshold={solar ? solar.total_capacity_kw * 0.85 : 85}
			/>
		</div>
		<div class="gauge-card">
			<GaugeChart
				value={solar ? solar.efficiency : 0}
				min={0}
				max={30}
				label="Efficiency"
				unit="%"
				size={140}
				color="#8b5cf6"
			/>
		</div>
		<div class="gauge-card">
			<GaugeChart
				value={water ? water.pressure : 0}
				min={0}
				max={10}
				label="Water Pressure"
				unit="bar"
				size={140}
				color="#22c55e"
				warningThreshold={6}
				dangerThreshold={8}
			/>
		</div>
	</div>

	<!-- Quick Actions -->
	<div class="quick-actions-section">
		<h3>Quick Actions</h3>
		<div class="quick-actions-grid">
			<a href="/water" class="quick-action water-action">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/></svg>
				<span>Water Controls</span>
			</a>
			<a href="/solar" class="quick-action solar-action">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/></svg>
				<span>Solar Controls</span>
			</a>
			<a href="/alarms" class="quick-action alarm-action">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>
				<span>View Alarms</span>
				{#if totalAlarms > 0}
					<span class="qa-badge">{totalAlarms}</span>
				{/if}
			</a>
			<a href="/history" class="quick-action history-action">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
				<span>View History</span>
			</a>
		</div>
		{#if quickActionFeedback}
			<div class="qa-feedback">{quickActionFeedback}</div>
		{/if}
	</div>

	<!-- Alarm Summary Cards -->
	<div class="alarm-summary">
		<div class="alarm-sum-card critical">
			<div class="asc-count">{$alarmCounts?.critical || 0}</div>
			<div class="asc-label">Critical</div>
		</div>
		<div class="alarm-sum-card warning">
			<div class="asc-count">{$alarmCounts?.warning || 0}</div>
			<div class="asc-label">Warning</div>
		</div>
		<div class="alarm-sum-card info">
			<div class="asc-count">{$alarmCounts?.info || 0}</div>
			<div class="asc-label">Info</div>
		</div>
	</div>
</div>

<style>
	.dashboard {
		max-width: 1400px;
		margin: 0 auto;
	}

	.dash-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 16px;
	}
	.dash-header h2 {
		font-size: 1.5rem;
		font-weight: 700;
	}
	.dash-date {
		font-size: 0.8rem;
		color: var(--text-muted);
	}
	.header-right {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.clock {
		font-family: var(--font-mono);
		font-size: 1.1rem;
		color: var(--text-secondary);
	}

	/* KPI Grid */
	.kpi-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 14px;
		margin-bottom: 18px;
	}

	/* Overview Grid */
	.overview-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 16px;
		margin-bottom: 18px;
	}

	/* Trend Charts */
	.trend-section {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 16px;
		margin-bottom: 18px;
	}
	.trend-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
	}

	/* Gauges Row */
	.gauges-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 14px;
		margin-bottom: 18px;
	}
	.gauge-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		display: flex;
		justify-content: center;
		align-items: center;
	}

	/* Quick Actions */
	.quick-actions-section {
		margin-bottom: 18px;
	}
	.quick-actions-section h3 {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin-bottom: 10px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.quick-actions-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 10px;
	}
	.quick-action {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 14px 16px;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		text-decoration: none;
		color: var(--text-secondary);
		font-size: 0.85rem;
		font-weight: 500;
		transition: all 0.15s;
		position: relative;
	}
	.quick-action:hover {
		background: var(--bg-card-hover);
		color: var(--text-primary);
		transform: translateY(-1px);
	}
	.water-action:hover { border-color: rgba(6, 182, 212, 0.4); color: var(--color-water); }
	.solar-action:hover { border-color: rgba(245, 158, 11, 0.4); color: var(--color-solar); }
	.alarm-action:hover { border-color: rgba(239, 68, 68, 0.4); color: var(--color-danger); }
	.history-action:hover { border-color: rgba(139, 92, 246, 0.4); color: var(--color-power); }
	.qa-badge {
		position: absolute;
		top: 8px;
		right: 8px;
		background: var(--color-danger);
		color: white;
		font-size: 0.6rem;
		font-weight: 700;
		padding: 1px 6px;
		border-radius: 8px;
	}
	.qa-feedback {
		margin-top: 8px;
		font-size: 0.8rem;
		color: var(--color-success);
		font-family: var(--font-mono);
	}

	/* Alarm Summary */
	.alarm-summary {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
		margin-bottom: 20px;
	}
	.alarm-sum-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		text-align: center;
	}
	.alarm-sum-card.critical {
		border-color: rgba(239, 68, 68, 0.3);
	}
	.alarm-sum-card.warning {
		border-color: rgba(245, 158, 11, 0.3);
	}
	.alarm-sum-card.info {
		border-color: rgba(59, 130, 246, 0.3);
	}
	.asc-count {
		font-family: var(--font-mono);
		font-size: 2rem;
		font-weight: 800;
	}
	.critical .asc-count { color: #ef4444; }
	.warning .asc-count { color: #f59e0b; }
	.info .asc-count { color: #3b82f6; }
	.asc-label {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-top: 4px;
	}

	/* Responsive */
	@media (max-width: 1200px) {
		.kpi-grid { grid-template-columns: repeat(2, 1fr); }
		.gauges-row { grid-template-columns: repeat(2, 1fr); }
		.quick-actions-grid { grid-template-columns: repeat(2, 1fr); }
	}
	@media (max-width: 768px) {
		.kpi-grid,
		.overview-grid,
		.trend-section,
		.gauges-row,
		.quick-actions-grid,
		.alarm-summary {
			grid-template-columns: 1fr;
		}
		.dash-header {
			flex-direction: column;
			gap: 4px;
		}
	}
</style>
