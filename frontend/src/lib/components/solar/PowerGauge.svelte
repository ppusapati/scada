<script>
	import GaugeChart from '../charts/GaugeChart.svelte';

	export let currentOutput = 0;
	export let totalCapacity = 100;
	export let dailyEnergy = 0;
	export let totalEnergy = 0;

	$: outputPercent = totalCapacity > 0 ? (currentOutput / totalCapacity) * 100 : 0;
</script>

<div class="power-gauge-panel">
	<div class="gauge-main">
		<GaugeChart
			value={currentOutput}
			min={0}
			max={totalCapacity}
			label="Current Output"
			unit="kW"
			color="#f59e0b"
			warningThreshold={totalCapacity * 0.85}
			dangerThreshold={totalCapacity * 0.95}
			size={180}
			decimals={1}
		/>
	</div>

	<div class="power-bar-section">
		<div class="bar-header">
			<span class="bar-label">Capacity Utilization</span>
			<span class="bar-value" style="color: var(--color-solar)">{outputPercent.toFixed(1)}%</span>
		</div>
		<div class="capacity-bar">
			<div
				class="capacity-fill"
				style="width: {Math.min(outputPercent, 100)}%"
				class:high={outputPercent > 85}
				class:over={outputPercent > 95}
			></div>
			<div class="capacity-markers">
				<div class="marker" style="left: 50%"><span>50%</span></div>
				<div class="marker warn" style="left: 85%"><span>85%</span></div>
			</div>
		</div>
	</div>

	<div class="energy-stats">
		<div class="energy-stat">
			<div class="es-label">Today</div>
			<div class="es-value">{dailyEnergy.toFixed(1)}</div>
			<div class="es-unit">kWh</div>
		</div>
		<div class="energy-divider"></div>
		<div class="energy-stat">
			<div class="es-label">Lifetime</div>
			<div class="es-value">{totalEnergy.toFixed(1)}</div>
			<div class="es-unit">MWh</div>
		</div>
	</div>
</div>

<style>
	.power-gauge-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
	}
	.gauge-main {
		display: flex;
		justify-content: center;
	}
	.power-bar-section {
		width: 100%;
	}
	.bar-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		margin-bottom: 6px;
	}
	.bar-label {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.bar-value {
		font-family: var(--font-mono);
		font-size: 0.9rem;
		font-weight: 700;
	}
	.capacity-bar {
		height: 10px;
		background: var(--bg-secondary);
		border-radius: 5px;
		overflow: visible;
		position: relative;
	}
	.capacity-fill {
		height: 100%;
		background: linear-gradient(90deg, #22c55e, #f59e0b);
		border-radius: 5px;
		transition: width 0.5s ease;
	}
	.capacity-fill.high {
		background: linear-gradient(90deg, #f59e0b, #ef4444);
	}
	.capacity-fill.over {
		background: #ef4444;
		animation: pulse 1s infinite;
	}
	@keyframes pulse { 0%,100% { opacity:1 } 50% { opacity:0.7 } }
	.capacity-markers {
		position: absolute;
		inset: 0;
	}
	.marker {
		position: absolute;
		top: -2px;
		width: 1px;
		height: 14px;
		background: var(--text-muted);
		opacity: 0.5;
	}
	.marker span {
		position: absolute;
		top: 16px;
		left: 50%;
		transform: translateX(-50%);
		font-size: 0.55rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.marker.warn {
		background: var(--color-warning);
		opacity: 0.7;
	}

	.energy-stats {
		display: flex;
		align-items: center;
		gap: 20px;
		width: 100%;
		padding: 12px;
		background: var(--bg-secondary);
		border-radius: 6px;
	}
	.energy-stat {
		flex: 1;
		text-align: center;
	}
	.es-label {
		font-size: 0.6rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 4px;
	}
	.es-value {
		font-family: var(--font-mono);
		font-size: 1.3rem;
		font-weight: 800;
		color: var(--color-solar);
	}
	.es-unit {
		font-size: 0.6rem;
		color: var(--text-muted);
	}
	.energy-divider {
		width: 1px;
		height: 40px;
		background: var(--border-color);
	}
</style>
