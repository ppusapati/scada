<script>
	import { onMount } from 'svelte';
	import { getWaterDevices, getSolarDevices, getReadingHistory, getAggregatedReadings } from '$lib/services/api';
	import LineChart from '$lib/components/charts/LineChart.svelte';
	import DateRangePicker from '$lib/components/common/DateRangePicker.svelte';

	let devices = [];
	let selectedDevice = '';
	let selectedMetrics = ['temperature'];
	let selectedInterval = '1 hour';
	let timeRange = '24h';
	let readings = [];
	let aggregated = [];
	let loading = false;
	let viewMode = 'raw';
	let error = '';
	let chartData = [];
	let customFrom = '';
	let customTo = '';
	let useCustomRange = false;

	const metrics = [
		{ value: 'temperature', label: 'Temperature', unit: 'C' },
		{ value: 'pressure', label: 'Pressure', unit: 'bar' },
		{ value: 'flow_rate', label: 'Flow Rate', unit: 'L/min' },
		{ value: 'flow_rate_in', label: 'Flow Rate In', unit: 'L/min' },
		{ value: 'flow_rate_out', label: 'Flow Rate Out', unit: 'L/min' },
		{ value: 'tank_level', label: 'Tank Level', unit: '%' },
		{ value: 'ph_level', label: 'pH Level', unit: 'pH' },
		{ value: 'turbidity', label: 'Turbidity', unit: 'NTU' },
		{ value: 'chlorine_level', label: 'Chlorine', unit: 'mg/L' },
		{ value: 'voltage', label: 'Voltage', unit: 'V' },
		{ value: 'current', label: 'Current', unit: 'A' },
		{ value: 'power', label: 'Power', unit: 'kW' },
		{ value: 'irradiance', label: 'Irradiance', unit: 'W/m2' },
		{ value: 'energy', label: 'Energy', unit: 'kWh' },
		{ value: 'frequency', label: 'Frequency', unit: 'Hz' },
		{ value: 'efficiency', label: 'Efficiency', unit: '%' },
	];

	const intervals = [
		{ value: '1 minute', label: '1 Min' },
		{ value: '5 minutes', label: '5 Min' },
		{ value: '15 minutes', label: '15 Min' },
		{ value: '30 minutes', label: '30 Min' },
		{ value: '1 hour', label: '1 Hour' },
		{ value: '6 hours', label: '6 Hours' },
		{ value: '1 day', label: '1 Day' },
	];

	const timeRanges = [
		{ value: '1h', label: 'Last Hour' },
		{ value: '6h', label: 'Last 6 Hours' },
		{ value: '24h', label: 'Last 24 Hours' },
		{ value: '7d', label: 'Last 7 Days' },
		{ value: '30d', label: 'Last 30 Days' },
	];

	onMount(async () => {
		try {
			const [waterDevices, solarDevices] = await Promise.all([
				getWaterDevices().catch(() => []),
				getSolarDevices().catch(() => [])
			]);
			devices = [...(waterDevices || []), ...(solarDevices || [])];
			if (devices.length > 0) {
				selectedDevice = devices[0].id;
			}
		} catch (e) {
			error = 'Failed to load devices. Using mock data.';
			// Mock devices for demo
			devices = [
				{ id: 'water-tank-001', name: 'Main Water Tank', subsystem: 'water', device_type: 'tank', location: 'Plant A', status: 'online', last_seen: new Date().toISOString(), metadata: {} },
				{ id: 'water-pump-001', name: 'Feed Pump #1', subsystem: 'water', device_type: 'pump', location: 'Plant A', status: 'online', last_seen: new Date().toISOString(), metadata: {} },
				{ id: 'solar-inv-001', name: 'Inverter Alpha', subsystem: 'solar', device_type: 'inverter', location: 'Field B', status: 'online', last_seen: new Date().toISOString(), metadata: {} },
				{ id: 'solar-panel-001', name: 'Panel Array 1', subsystem: 'solar', device_type: 'panel', location: 'Field B', status: 'online', last_seen: new Date().toISOString(), metadata: {} },
			];
			selectedDevice = devices[0].id;
		}
	});

	function getTimeRange() {
		if (useCustomRange && customFrom && customTo) {
			return { from: customFrom, to: customTo };
		}
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
		if (!selectedDevice || selectedMetrics.length === 0) {
			error = 'Please select a device and at least one metric.';
			return;
		}
		loading = true;
		error = '';
		const { from, to } = getTimeRange();

		try {
			const metric = selectedMetrics[0]; // Primary metric for query
			if (viewMode === 'raw') {
				readings = await getReadingHistory(selectedDevice, {
					metric, from, to, limit: 1000
				});
				aggregated = [];
				chartData = (readings || []).map(r => ({ timestamp: r.timestamp, value: r.value }));
			} else {
				aggregated = await getAggregatedReadings(selectedDevice, {
					metric, interval: selectedInterval, from, to
				});
				readings = [];
				chartData = (aggregated || []).map(r => ({ timestamp: r.timestamp, value: r.avg_value }));
			}
		} catch (e) {
			error = `Failed to fetch data: ${e.message}. Showing mock data.`;
			// Generate mock data for demo
			chartData = generateMockHistory(60);
			if (viewMode === 'raw') {
				readings = chartData.map((d, i) => ({
					id: i,
					device_id: selectedDevice,
					timestamp: d.timestamp,
					metric: selectedMetrics[0],
					value: d.value,
					unit: metrics.find(m => m.value === selectedMetrics[0])?.unit || '',
					quality: 0
				}));
			} else {
				aggregated = chartData.map((d, i) => ({
					timestamp: d.timestamp,
					avg_value: d.value,
					min_value: d.value * 0.9,
					max_value: d.value * 1.1,
					sample_count: 12
				}));
			}
		} finally {
			loading = false;
		}
	}

	function generateMockHistory(count) {
		const data = [];
		const now = Date.now();
		const base = 50 + Math.random() * 50;
		for (let i = count - 1; i >= 0; i--) {
			data.push({
				timestamp: new Date(now - i * 5 * 60 * 1000).toISOString(),
				value: base + (Math.random() - 0.5) * 20
			});
		}
		return data;
	}

	function handleDateChange(e) {
		customFrom = e.detail.from;
		customTo = e.detail.to;
		useCustomRange = true;
	}

	function toggleMetric(metric) {
		if (selectedMetrics.includes(metric)) {
			if (selectedMetrics.length > 1) {
				selectedMetrics = selectedMetrics.filter(m => m !== metric);
			}
		} else {
			selectedMetrics = [...selectedMetrics, metric];
		}
	}

	function formatTimestamp(ts) {
		return new Date(ts).toLocaleString([], {
			month: 'short', day: 'numeric',
			hour: '2-digit', minute: '2-digit', second: '2-digit'
		});
	}

	function exportCSV() {
		let csv = '';
		if (viewMode === 'raw' && readings.length > 0) {
			csv = 'Timestamp,Metric,Value,Unit,Quality\n';
			for (const r of readings) {
				csv += `"${r.timestamp}","${r.metric}",${r.value},"${r.unit}",${r.quality}\n`;
			}
		} else if (viewMode === 'aggregated' && aggregated.length > 0) {
			csv = 'Timestamp,Average,Min,Max,Samples\n';
			for (const r of aggregated) {
				csv += `"${r.timestamp}",${r.avg_value.toFixed(3)},${r.min_value.toFixed(3)},${r.max_value.toFixed(3)},${r.sample_count}\n`;
			}
		}
		if (!csv) return;

		const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = `scada-${selectedDevice}-${selectedMetrics[0]}-${new Date().toISOString().slice(0,10)}.csv`;
		link.click();
		URL.revokeObjectURL(url);
	}

	$: currentMetricInfo = metrics.find(m => m.value === selectedMetrics[0]) || { label: '', unit: '' };
	$: hasData = (viewMode === 'raw' && readings.length > 0) || (viewMode === 'aggregated' && aggregated.length > 0);
</script>

<div class="history-page">
	<header class="page-header">
		<div class="header-title">
			<h2>
				<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:inline-block; vertical-align:middle; margin-right:8px; color:var(--color-power)">
					<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
				</svg>
				Historical Data
			</h2>
		</div>
		{#if hasData}
			<button class="btn btn-primary" on:click={exportCSV}>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
				Export CSV
			</button>
		{/if}
	</header>

	<!-- Query Panel -->
	<div class="query-panel">
		<div class="query-row">
			<div class="query-field">
				<label for="hist-device">Device</label>
				<select id="hist-device" bind:value={selectedDevice}>
					{#each devices as device}
						<option value={device.id}>
							{device.name} ({device.subsystem})
						</option>
					{/each}
				</select>
			</div>

			<div class="query-field">
				<label>Primary Metric</label>
				<select bind:value={selectedMetrics[0]}>
					{#each metrics as metric}
						<option value={metric.value}>{metric.label} ({metric.unit})</option>
					{/each}
				</select>
			</div>

			<div class="query-field">
				<label for="hist-view">View Mode</label>
				<select id="hist-view" bind:value={viewMode}>
					<option value="raw">Raw Data</option>
					<option value="aggregated">Aggregated</option>
				</select>
			</div>

			{#if viewMode === 'aggregated'}
				<div class="query-field">
					<label for="hist-interval">Aggregation</label>
					<select id="hist-interval" bind:value={selectedInterval}>
						{#each intervals as interval}
							<option value={interval.value}>{interval.label}</option>
						{/each}
					</select>
				</div>
			{/if}
		</div>

		<div class="query-row">
			<div class="query-field range-field">
				<label>Time Range</label>
				<div class="range-buttons">
					{#each timeRanges as range}
						<button
							class="range-btn"
							class:active={timeRange === range.value && !useCustomRange}
							on:click={() => { timeRange = range.value; useCustomRange = false; }}
						>
							{range.label}
						</button>
					{/each}
					<button
						class="range-btn"
						class:active={useCustomRange}
						on:click={() => { useCustomRange = true; }}
					>
						Custom
					</button>
				</div>
			</div>

			{#if useCustomRange}
				<div class="query-field">
					<DateRangePicker presets={false} on:change={handleDateChange} />
				</div>
			{/if}

			<div class="query-field query-action">
				<button class="btn btn-primary query-btn" on:click={fetchData} disabled={loading}>
					{#if loading}
						<span class="spinner-sm"></span> Querying...
					{:else}
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
						Query Data
					{/if}
				</button>
			</div>
		</div>

		<!-- Metric multi-select pills -->
		<div class="metric-pills">
			<span class="pills-label">Metrics:</span>
			{#each metrics as metric}
				<button
					class="pill"
					class:selected={selectedMetrics.includes(metric.value)}
					on:click={() => toggleMetric(metric.value)}
				>
					{metric.label}
				</button>
			{/each}
		</div>
	</div>

	<!-- Error Message -->
	{#if error}
		<div class="error-bar">
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
			{error}
		</div>
	{/if}

	<!-- Chart -->
	{#if chartData.length > 0}
		<div class="chart-panel">
			<LineChart
				data={chartData}
				height={280}
				color={currentMetricInfo.label.includes('Solar') || currentMetricInfo.label.includes('Irradiance') || currentMetricInfo.label.includes('Power') || currentMetricInfo.label.includes('Energy') ? '#f59e0b' : '#3b82f6'}
				label="{currentMetricInfo.label} - {selectedDevice}"
				unit={currentMetricInfo.unit}
				showDots={chartData.length < 50}
			/>
		</div>
	{/if}

	<!-- Data Table -->
	<div class="results-panel">
		<div class="results-header">
			<span class="results-title">
				{viewMode === 'raw' ? 'Raw Readings' : 'Aggregated Data'}
			</span>
			<span class="results-count">
				{viewMode === 'raw' ? readings.length : aggregated.length} records
			</span>
		</div>

		<div class="table-scroll">
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
						{#each readings as reading}
							<tr>
								<td class="mono">{formatTimestamp(reading.timestamp)}</td>
								<td class="metric-name">{reading.metric.replace(/_/g, ' ')}</td>
								<td class="mono value">{reading.value.toFixed(3)}</td>
								<td class="unit">{reading.unit}</td>
								<td>
									{#if reading.quality === 0}
										<span class="quality good">Good</span>
									{:else if reading.quality === 1}
										<span class="quality uncertain">Uncertain</span>
									{:else}
										<span class="quality bad">Bad</span>
									{/if}
								</td>
							</tr>
						{:else}
							<tr><td colspan="5" class="empty">No data. Select a device and click Query.</td></tr>
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
							<th>Range</th>
							<th>Samples</th>
						</tr>
					</thead>
					<tbody>
						{#each aggregated as row}
							<tr>
								<td class="mono">{formatTimestamp(row.timestamp)}</td>
								<td class="mono value">{row.avg_value.toFixed(3)}</td>
								<td class="mono">{row.min_value.toFixed(3)}</td>
								<td class="mono">{row.max_value.toFixed(3)}</td>
								<td class="mono range">{(row.max_value - row.min_value).toFixed(3)}</td>
								<td class="mono samples">{row.sample_count}</td>
							</tr>
						{:else}
							<tr><td colspan="6" class="empty">No aggregated data</td></tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	</div>
</div>

<style>
	.history-page { max-width: 1400px; margin: 0 auto; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 16px;
		flex-wrap: wrap;
		gap: 12px;
	}
	.header-title h2 { color: var(--color-power); font-size: 1.3rem; display: flex; align-items: center; }

	/* Query Panel */
	.query-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 16px;
	}
	.query-row {
		display: flex;
		gap: 14px;
		align-items: flex-end;
		flex-wrap: wrap;
		margin-bottom: 12px;
	}
	.query-row:last-child { margin-bottom: 0; }
	.query-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 140px;
	}
	.query-field label {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-weight: 600;
	}
	.query-field select {
		background: var(--bg-secondary);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 8px 12px;
		border-radius: 6px;
		font-size: 0.8rem;
	}
	.query-field select:focus {
		outline: none;
		border-color: var(--color-info);
	}
	.query-action {
		margin-left: auto;
	}
	.query-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		white-space: nowrap;
	}
	.spinner-sm {
		width: 14px;
		height: 14px;
		border: 2px solid rgba(255,255,255,0.3);
		border-top-color: white;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin { to { transform: rotate(360deg); } }

	.range-field {
		flex: 1;
		min-width: 300px;
	}
	.range-buttons {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.range-btn {
		padding: 6px 12px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.72rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;
	}
	.range-btn:hover { background: var(--bg-card-hover); }
	.range-btn.active { background: var(--color-info); color: white; border-color: var(--color-info); }

	.metric-pills {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
		padding-top: 10px;
		border-top: 1px solid var(--border-color);
	}
	.pills-label {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-right: 4px;
	}
	.pill {
		padding: 3px 10px;
		border: 1px solid var(--border-color);
		border-radius: 12px;
		background: transparent;
		color: var(--text-muted);
		font-size: 0.65rem;
		cursor: pointer;
		transition: all 0.15s;
	}
	.pill:hover { color: var(--text-primary); background: var(--bg-card-hover); }
	.pill.selected { background: rgba(59,130,246,0.15); color: var(--color-info); border-color: rgba(59,130,246,0.3); }

	/* Error */
	.error-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		background: rgba(245, 158, 11, 0.1);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: 6px;
		margin-bottom: 14px;
		font-size: 0.8rem;
		color: var(--color-warning);
	}

	/* Chart */
	.chart-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 16px;
	}

	/* Results */
	.results-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}
	.results-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-color);
	}
	.results-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-secondary);
	}
	.results-count {
		font-size: 0.7rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.table-scroll {
		overflow: auto;
		max-height: calc(100vh - 500px);
		min-height: 200px;
	}

	.data-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.data-table th {
		position: sticky;
		top: 0;
		background: var(--bg-secondary);
		padding: 10px 14px;
		text-align: left;
		font-size: 0.68rem;
		text-transform: uppercase;
		color: var(--text-muted);
		letter-spacing: 0.04em;
		border-bottom: 1px solid var(--border-color);
		z-index: 1;
	}
	.data-table td {
		padding: 8px 14px;
		border-bottom: 1px solid var(--border-color);
	}
	.data-table tbody tr:hover { background: var(--bg-card-hover); }
	.mono { font-family: var(--font-mono); font-size: 0.72rem; white-space: nowrap; }
	.value { color: var(--color-info); font-weight: 600; }
	.range { color: var(--color-warning); }
	.samples { color: var(--text-muted); }
	.metric-name { text-transform: capitalize; }
	.unit { font-size: 0.72rem; color: var(--text-muted); }
	.quality {
		font-size: 0.6rem;
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 3px;
		text-transform: uppercase;
	}
	.quality.good { background: rgba(34,197,94,0.15); color: #22c55e; }
	.quality.uncertain { background: rgba(245,158,11,0.15); color: #f59e0b; }
	.quality.bad { background: rgba(239,68,68,0.15); color: #ef4444; }
	.empty { text-align: center; padding: 40px; color: var(--text-muted); }

	/* Responsive */
	@media (max-width: 768px) {
		.query-row { flex-direction: column; }
		.query-field { min-width: 100%; }
		.query-action { margin-left: 0; width: 100%; }
		.query-btn { width: 100%; justify-content: center; }
		.range-field { min-width: 100%; }
	}
</style>
