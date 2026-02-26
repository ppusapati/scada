<script>
	export let alarms = [];

	$: criticalAlarms = alarms.filter(a => a.severity === 'critical' && a.status === 'active');
	$: hasAlarms = criticalAlarms.length > 0;
</script>

{#if hasAlarms}
	<div class="alarm-ticker">
		<div class="ticker-label">ACTIVE ALARMS</div>
		<div class="ticker-scroll">
			{#each criticalAlarms as alarm}
				<span class="ticker-item">
					<span class="badge badge-critical">{alarm.severity}</span>
					{alarm.message}
					<span class="ticker-time">
						{new Date(alarm.occurred_at).toLocaleTimeString()}
					</span>
				</span>
			{/each}
		</div>
	</div>
{/if}

<style>
	.alarm-ticker {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 6px;
		padding: 8px 16px;
		margin-bottom: 20px;
		display: flex;
		align-items: center;
		gap: 16px;
		overflow: hidden;
	}

	.ticker-label {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--color-danger);
		white-space: nowrap;
		animation: pulse 1.5s infinite;
	}

	.ticker-scroll {
		display: flex;
		gap: 24px;
		overflow-x: auto;
		flex: 1;
	}

	.ticker-item {
		display: flex;
		align-items: center;
		gap: 8px;
		white-space: nowrap;
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	.ticker-time {
		color: var(--text-muted);
		font-size: 0.7rem;
		font-family: var(--font-mono);
	}
</style>
