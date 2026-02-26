<script>
	export let label = '';
	export let value = '--';
	export let unit = '';
	export let color = 'var(--color-info)';
	export let icon = '';
	export let trend = 'normal'; // normal, low, critical

	const icons = {
		water: '&#9679;',
		solar: '&#9788;',
		alarm: '&#9888;',
		power: '&#9889;',
	};
</script>

<div class="metric-card" style="--accent: {color}" class:critical={trend === 'critical'} class:low={trend === 'low'}>
	<div class="card-header">
		<span class="card-icon">{@html icons[icon] || '&#9632;'}</span>
		<span class="metric-label">{label}</span>
	</div>
	<div class="card-body">
		<span class="metric-value" style="color: {color}">{value}</span>
		{#if unit}
			<span class="metric-unit">{unit}</span>
		{/if}
	</div>
	{#if trend === 'critical'}
		<div class="trend-bar critical-bar"></div>
	{:else if trend === 'low'}
		<div class="trend-bar warning-bar"></div>
	{:else}
		<div class="trend-bar normal-bar"></div>
	{/if}
</div>

<style>
	.metric-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		position: relative;
		overflow: hidden;
		transition: all 0.2s;
	}
	.metric-card:hover {
		background: var(--bg-card-hover);
		transform: translateY(-1px);
	}
	.metric-card.critical {
		border-color: rgba(239, 68, 68, 0.4);
		animation: pulse 2s infinite;
	}

	.card-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
	}

	.card-icon {
		font-size: 1rem;
		color: var(--accent);
	}

	.card-body {
		display: flex;
		align-items: baseline;
	}

	.trend-bar {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 3px;
	}
	.normal-bar { background: var(--color-success); }
	.warning-bar { background: var(--color-warning); }
	.critical-bar { background: var(--color-danger); animation: pulse 1.5s infinite; }
</style>
