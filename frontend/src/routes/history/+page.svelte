<script>
	import { onMount } from 'svelte';
	import { getWaterDevices, getSolarDevices, getReadingHistory, getAggregatedReadings } from '$lib/services/api';

	let devices = [];
	let selectedDevice = '';
	let selectedMetric = 'temperature';
	let selectedInterval = '1 hour';
	let timeRange = '24h';
	let readings = [];
	let aggregated = [];
	let loading = false;
	let viewMode = 'raw'; // raw or aggregated

	const metrics = [
		'temperature', 'pressure', 'flow_rate', 'flow_rate_in', 'flow_rate_out',
		'tank_level', 'ph_level', 'turbidity', 'chlorine_level',
		'voltage', 'current', 'power', 'irradiance', 'energy', 'frequency', 'efficiency'
	];

	const intervals = ['1 minute', '5 minutes', '15 minutes', '30 minutes', '1 hour', '6 hours', '1 day'];

	onMount(async () => {
		try {
			const [waterDevices, solarDevices] = await Promise.all([
				getWaterDevices(),
				getSolarDevices()
			]);
			devices = [...(waterDevices || []), ...(solarDevices || [])];
			if (devices.length > 0) {
				selectedDevice = devices[0].id;
			}
		} catch (e) {
			console.error('Failed to load devices:', e);
		}
	});

	function getTimeRange() {
		const now = new Date();
		const ranges = {
			'1h': 60 * 60 * 1000,
			'6h': 6 * 60 * 60 * 1000,
			'24h': 24 * 60 * 60 * 1000,
			'7d': 7 * 24 * 60 * 60 * 1000,
			'30d': 30 * 24 * 60 * 60 * 1000,
		};
		const from = new Date(now.getTime() - (ranges[timeRange] || ranges['24h']));
		return { from: from.toISOString(), to: now.toISOString() };
	}

	async function fetchData() {
		if (!selectedDevice) return;
		loading = true;
		const { from, to } = getTimeRange();

		try {
			if (viewMode === 'raw') {
				readings = await getReadingHistory(selectedDevice, {
					metric: selectedMetric, from, to, limit: 500
				});
			} else {
				aggregated = await getAggregatedReadings(selectedDevice, {
					metric: selectedMetric, interval: selectedInterval, from, to
				});
			}
		} catch (e) {
			console.error('Failed to fetch data:', e);
			readings = [];
			aggregated = [];
		} finally {
			loading = false;
		}
	}

	function formatTimestamp(ts) {
		return new Date(ts).toLocaleString();
	}
</script>

<div class="history-page">
	<header class="page-header">
		<h2>Historical Data</h2>
	</header>

	<!-- Query Controls -->
	<div class="query-panel card">
		<div class="query-row">
			<div class="query-field">
				<label>Device</label>
				<select bind:value={selectedDevice}>
					{#each devices as device}
						<option value={device.id}>{device.name} ({device.subsystem})</option>
					{/each}
				</select>
			</div>
			<div class="query-field">
				<label>Metric</label>
				<select bind:value={selectedMetric}>
					{#each metrics as metric}
						<option value={metric}>{metric.replace(/_/g, ' ')}</option>
					{/each}
				</select>
			</div>
			<div class="query-field">
				<label>Time Range</label>
				<select bind:value={timeRange}>
					<option value="1h">Last Hour</option>
					<option value="6h">Last 6 Hours</option>
					<option value="24h">Last 24 Hours</option>
					<option value="7d">Last 7 Days</option>
					<option value="30d">Last 30 Days</option>
				</select>
			</div>
			<div class="query-field">
				<label>View</label>
				<select bind:value={viewMode}>
					<option value="raw">Raw Data</option>
					<option value="aggregated">Aggregated</option>
				</select>
			</div>
			{#if viewMode === 'aggregated'}
				<div class="query-field">
					<label>Interval</label>
					<select bind:value={selectedInterval}>
						{#each intervals as interval}
							<option value={interval}>{interval}</option>
						{/each}
					</select>
				</div>
			{/if}
			<div class="query-field query-action">
				<button class="btn btn-primary" on:click={fetchData} disabled={loading}>
					{loading ? 'Loading...' : 'Query'}
				</button>
			</div>
		</div>
	</div>

	<!-- Results Table -->
	<div class="results-container card">
		{#if viewMode === 'raw'}
			<table class="data-table">
				<thead>
					<tr>
						<th>Timestamp</th>
						<th>Metric</th>
						<th>Value</th>
						<th>Unit</th>
						<th>Quality</th>
					</tr>
				</thead>
				<tbody>
					{#each readings || [] as reading}
						<tr>
							<td class="mono">{formatTimestamp(reading.timestamp)}</td>
							<td>{reading.metric}</td>
							<td class="mono value">{reading.value.toFixed(3)}</td>
							<td>{reading.unit}</td>
							<td>
								<span class="badge" class:badge-success={reading.quality === 0}
									class:badge-warning={reading.quality === 1}
									class:badge-critical={reading.quality === 2}>
									{reading.quality === 0 ? 'Good' : reading.quality === 1 ? 'Uncertain' : 'Bad'}
								</span>
							</td>
						</tr>
					{:else}
						<tr><td colspan="5" class="empty">No data — select a device and click Query</td></tr>
					{/each}
				</tbody>
			</table>
		{:else}
			<table class="data-table">
				<thead>
					<tr>
						<th>Time Bucket</th>
						<th>Average</th>
						<th>Min</th>
						<th>Max</th>
						<th>Samples</th>
					</tr>
				</thead>
				<tbody>
					{#each aggregated || [] as row}
						<tr>
							<td class="mono">{formatTimestamp(row.timestamp)}</td>
							<td class="mono value">{row.avg_value.toFixed(3)}</td>
							<td class="mono">{row.min_value.toFixed(3)}</td>
							<td class="mono">{row.max_value.toFixed(3)}</td>
							<td>{row.sample_count}</td>
						</tr>
					{:else}
						<tr><td colspan="5" class="empty">No aggregated data</td></tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</div>

<style>
	.history-page { max-width: 1400px; margin: 0 auto; }
	.page-header { margin-bottom: 20px; }

	.query-panel { margin-bottom: 16px; }
	.query-row { display: flex; gap: 16px; align-items: flex-end; flex-wrap: wrap; }
	.query-field { display: flex; flex-direction: column; gap: 4px; }
	.query-field label { font-size: 0.7rem; color: var(--text-muted); text-transform: uppercase; }
	.query-field select {
		background: var(--bg-secondary); color: var(--text-primary);
		border: 1px solid var(--border-color); padding: 8px 12px;
		border-radius: 6px; font-size: 0.85rem;
	}
	.query-action { margin-left: auto; }

	.results-container { overflow: auto; max-height: calc(100vh - 280px); padding: 0; }

	.data-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.data-table th {
		position: sticky; top: 0;
		background: var(--bg-secondary); padding: 10px 12px;
		text-align: left; font-size: 0.7rem;
		text-transform: uppercase; color: var(--text-muted);
		border-bottom: 1px solid var(--border-color);
	}
	.data-table td { padding: 8px 12px; border-bottom: 1px solid var(--border-color); }
	.mono { font-family: var(--font-mono); font-size: 0.75rem; }
	.value { color: var(--color-info); font-weight: 600; }
	.empty { text-align: center; padding: 40px; color: var(--text-muted); }
</style>
