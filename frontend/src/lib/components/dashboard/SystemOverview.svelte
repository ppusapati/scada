<script>
	export let title = '';
	export let subsystem = 'water';
	export let metrics = [];
	export let status = 'unknown';
	export let mode = 'auto';

	$: statusColor = {
		running: 'var(--color-success)',
		online: 'var(--color-success)',
		stopped: 'var(--color-offline)',
		offline: 'var(--color-offline)',
		fault: 'var(--color-danger)',
	}[status] || 'var(--color-offline)';

	$: accentColor = subsystem === 'water' ? 'var(--color-water)' : 'var(--color-solar)';
</script>

<div class="system-panel">
	<div class="panel-header">
		<div class="panel-title-row">
			<h3 style="color: {accentColor}">{title}</h3>
			<div class="status-badges">
				<span class="badge" style="background: {statusColor}20; color: {statusColor}; border: 1px solid {statusColor}40;">
					{status}
				</span>
				<span class="badge badge-info">{mode}</span>
			</div>
		</div>
	</div>

	<div class="metrics-grid">
		{#each metrics as metric}
			<div class="mini-metric">
				<div class="mini-label">{metric.label}</div>
				<div class="mini-value">
					<span style="color: {accentColor}">{typeof metric.value === 'number' ? metric.value.toFixed(1) : metric.value}</span>
					<span class="mini-unit">{metric.unit}</span>
				</div>
			</div>
		{/each}
	</div>

	<div class="panel-footer">
		<a href="/{subsystem}" class="btn" style="font-size: 0.75rem;">
			View Details →
		</a>
	</div>
</div>

<style>
	.system-panel {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}

	.panel-header {
		padding: 16px;
		border-bottom: 1px solid var(--border-color);
	}

	.panel-title-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.panel-title-row h3 {
		font-size: 1rem;
		font-weight: 600;
	}

	.status-badges {
		display: flex;
		gap: 6px;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1px;
		background: var(--border-color);
	}

	.mini-metric {
		background: var(--bg-card);
		padding: 14px;
		text-align: center;
	}
	.mini-metric:hover {
		background: var(--bg-card-hover);
	}

	.mini-label {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 6px;
	}

	.mini-value {
		font-family: var(--font-mono);
		font-size: 1.2rem;
		font-weight: 700;
	}

	.mini-unit {
		font-size: 0.6rem;
		color: var(--text-muted);
		margin-left: 2px;
	}

	.panel-footer {
		padding: 12px 16px;
		border-top: 1px solid var(--border-color);
		text-align: right;
	}
</style>
