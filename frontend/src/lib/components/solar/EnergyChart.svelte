<script>
	import BarChart from '../charts/BarChart.svelte';

	export let data = [];
	export let height = 220;

	// Generate mock 7-day data if none provided
	$: displayData = data.length > 0 ? data : generateMockData();

	function generateMockData() {
		const days = [];
		const now = new Date();
		for (let i = 6; i >= 0; i--) {
			const d = new Date(now);
			d.setDate(d.getDate() - i);
			const dayName = d.toLocaleDateString('en', { weekday: 'short' });
			const dateStr = d.toLocaleDateString('en', { month: 'short', day: 'numeric' });
			// Simulate production that varies by "weather" and day of week
			const baseProduction = 120 + Math.random() * 80;
			const weatherFactor = 0.5 + Math.random() * 0.5;
			days.push({
				label: `${dayName} ${dateStr}`,
				value: i === 0 ? baseProduction * weatherFactor * 0.6 : baseProduction * weatherFactor // today is partial
			});
		}
		return days;
	}

	$: totalWeek = displayData.reduce((sum, d) => sum + d.value, 0);
	$: avgDay = displayData.length > 0 ? totalWeek / displayData.length : 0;
	$: peakDay = displayData.length > 0 ? Math.max(...displayData.map(d => d.value)) : 0;
</script>

<div class="energy-chart-panel">
	<div class="chart-header">
		<h3>Daily Energy Production</h3>
		<div class="chart-stats">
			<span class="stat">
				<span class="stat-label">Weekly Total</span>
				<span class="stat-value">{totalWeek.toFixed(0)} kWh</span>
			</span>
			<span class="stat">
				<span class="stat-label">Daily Avg</span>
				<span class="stat-value">{avgDay.toFixed(0)} kWh</span>
			</span>
			<span class="stat">
				<span class="stat-label">Peak Day</span>
				<span class="stat-value">{peakDay.toFixed(0)} kWh</span>
			</span>
		</div>
	</div>
	<div class="chart-body">
		<BarChart
			data={displayData}
			{height}
			color="#f59e0b"
			gradientTo="#fcd34d"
			unit="kWh"
		/>
	</div>
</div>

<style>
	.energy-chart-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}
	.chart-header {
		padding: 14px 16px;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}
	.chart-header h3 {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--color-solar);
	}
	.chart-stats {
		display: flex;
		gap: 16px;
	}
	.stat {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.stat-label {
		font-size: 0.55rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.stat-value {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 700;
		color: var(--color-solar);
	}
	.chart-body {
		padding: 12px 16px 8px;
	}
</style>
