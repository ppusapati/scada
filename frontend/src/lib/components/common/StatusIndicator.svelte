<script>
	/** @type {'online' | 'offline' | 'fault' | 'maintenance' | 'unknown'} */
	export let status = 'unknown';
	export let label = '';
	export let size = 'sm';
	export let showLabel = true;
	export let pulse = true;

	const statusConfig = {
		online: { color: '#22c55e', text: 'Online' },
		offline: { color: '#6b7280', text: 'Offline' },
		fault: { color: '#ef4444', text: 'Fault' },
		maintenance: { color: '#f59e0b', text: 'Maintenance' },
		running: { color: '#22c55e', text: 'Running' },
		stopped: { color: '#6b7280', text: 'Stopped' },
		unknown: { color: '#6b7280', text: 'Unknown' },
		connected: { color: '#22c55e', text: 'Connected' },
		disconnected: { color: '#ef4444', text: 'Disconnected' },
	};

	$: config = statusConfig[status] || statusConfig.unknown;
	$: displayLabel = label || config.text;
	$: shouldPulse = pulse && (status === 'fault' || status === 'online' || status === 'running' || status === 'connected');
	$: dotSize = size === 'lg' ? '12px' : size === 'md' ? '10px' : '8px';
</script>

<div class="status-indicator" class:sm={size === 'sm'} class:md={size === 'md'} class:lg={size === 'lg'}>
	<span
		class="dot"
		class:pulse-anim={shouldPulse}
		style="width: {dotSize}; height: {dotSize}; background-color: {config.color}; box-shadow: 0 0 6px {shouldPulse ? config.color : 'transparent'};"
	></span>
	{#if showLabel}
		<span class="label">{displayLabel}</span>
	{/if}
</div>

<style>
	.status-indicator {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.dot {
		border-radius: 50%;
		flex-shrink: 0;
		transition: all 0.3s;
	}
	.pulse-anim {
		animation: statusPulse 2s infinite;
	}
	@keyframes statusPulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
	.label {
		font-size: 0.75rem;
		color: var(--text-secondary);
		white-space: nowrap;
	}
	.lg .label { font-size: 0.875rem; }
	.md .label { font-size: 0.8rem; }
</style>
