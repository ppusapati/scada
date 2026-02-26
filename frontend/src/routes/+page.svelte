<script>
	import { onMount, onDestroy } from 'svelte';
	import { waterSystems, solarSystems, activeAlarms, alarmCounts } from '$lib/stores/scada';
	import { wsConnected } from '$lib/services/websocket';
	import { auth } from '$lib/stores/auth';
	import MetricCard from '$lib/components/dashboard/MetricCard.svelte';
	import AlarmTicker from '$lib/components/dashboard/AlarmTicker.svelte';
	import SystemOverview from '$lib/components/dashboard/SystemOverview.svelte';

	$: water = $waterSystems[0];
	$: solar = $solarSystems[0];
	$: totalAlarms = $alarmCounts.critical + $alarmCounts.warning + $alarmCounts.info;

	let now = new Date();
	let clockInterval;

	onMount(() => {
		clockInterval = setInterval(() => { now = new Date(); }, 1000);
	});

	onDestroy(() => {
		if (clockInterval) clearInterval(clockInterval);
	});
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
			<span class="status-dot" class:online={$wsConnected} class:offline={!$wsConnected}></span>
		</div>
	</header>

	<!-- Alarm Ticker -->
	<AlarmTicker alarms={$activeAlarms} />

	<!-- KPI Cards Row -->
	<div class="grid-4" style="margin-bottom: 20px;">
		<MetricCard
			label="Water Tank Level"
			value={water?.tank_level ?? '--'}
			unit="%"
			color="var(--color-water)"
			icon="water"
			trend={water?.tank_level > 50 ? 'normal' : 'low'}
		/>
		<MetricCard
			label="Solar Output"
			value={solar?.current_output_kw?.toFixed(1) ?? '--'}
			unit="kW"
			color="var(--color-solar)"
			icon="solar"
			trend="normal"
		/>
		<MetricCard
			label="Active Alarms"
			value={totalAlarms}
			unit=""
			color={$alarmCounts.critical > 0 ? 'var(--color-danger)' : 'var(--color-success)'}
			icon="alarm"
			trend={$alarmCounts.critical > 0 ? 'critical' : 'normal'}
		/>
		<MetricCard
			label="Grid Frequency"
			value={solar?.grid_frequency?.toFixed(1) ?? '--'}
			unit="Hz"
			color="var(--color-power)"
			icon="power"
			trend="normal"
		/>
	</div>

	<!-- System Panels -->
	<div class="grid-2">
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
				{ label: 'Irradiance', value: solar.irradiance, unit: 'W/m²' },
				{ label: 'Panel Temp', value: solar.panel_temperature, unit: '°C' },
				{ label: 'Daily Energy', value: solar.daily_energy_kwh, unit: 'kWh' },
				{ label: 'Efficiency', value: solar.efficiency, unit: '%' },
			] : []}
			status={solar?.inverter_status ?? 'unknown'}
			mode={solar?.operating_mode ?? 'unknown'}
		/>
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
		margin-bottom: 20px;
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
</style>
