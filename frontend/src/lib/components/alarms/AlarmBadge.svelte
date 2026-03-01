<script>
	/** @type {'critical' | 'warning' | 'info'} */
	export let severity = 'info';
	export let compact = false;
	export let pulse = false;

	const config = {
		critical: { bg: 'rgba(239, 68, 68, 0.2)', color: '#ef4444', border: 'rgba(239, 68, 68, 0.3)', label: 'CRITICAL' },
		warning: { bg: 'rgba(245, 158, 11, 0.2)', color: '#f59e0b', border: 'rgba(245, 158, 11, 0.3)', label: 'WARNING' },
		info: { bg: 'rgba(59, 130, 246, 0.2)', color: '#3b82f6', border: 'rgba(59, 130, 246, 0.3)', label: 'INFO' },
	};

	$: c = config[severity] || config.info;
	$: shouldPulse = pulse || severity === 'critical';
</script>

<span
	class="alarm-badge"
	class:compact
	class:pulse-anim={shouldPulse}
	style="background: {c.bg}; color: {c.color}; border-color: {c.border};"
>
	{#if !compact}
		<span class="badge-dot" style="background: {c.color}"></span>
	{/if}
	{c.label}
</span>

<style>
	.alarm-badge {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 3px 10px;
		border-radius: 4px;
		font-size: 0.65rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		border: 1px solid;
		white-space: nowrap;
	}
	.alarm-badge.compact {
		padding: 2px 6px;
		font-size: 0.6rem;
		gap: 0;
	}
	.badge-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.pulse-anim {
		animation: badgePulse 1.5s infinite;
	}
	@keyframes badgePulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.6; }
	}
</style>
