<script>
	import { createEventDispatcher } from 'svelte';

	export let pumpId = 'P-001';
	export let pumpName = 'Main Pump';
	/** @type {'running' | 'stopped' | 'fault'} */
	export let status = 'stopped';
	export let flowRate = 0;
	export let runtime = 0;
	export let disabled = false;
	export let loading = false;

	const dispatch = createEventDispatcher();

	$: isRunning = status === 'running';
	$: isFault = status === 'fault';
	$: statusColor = isRunning ? '#22c55e' : isFault ? '#ef4444' : '#6b7280';

	function formatRuntime(seconds) {
		const hours = Math.floor(seconds / 3600);
		const mins = Math.floor((seconds % 3600) / 60);
		return `${hours}h ${mins}m`;
	}

	function handleStart() { dispatch('command', { action: 'start' }); }
	function handleStop() { dispatch('command', { action: 'stop' }); }
</script>

<div class="pump-control" class:running={isRunning} class:fault={isFault}>
	<div class="pump-header">
		<div class="pump-icon" style="border-color: {statusColor}">
			<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke={statusColor} stroke-width="2">
				<circle cx="12" cy="12" r="6" />
				{#if isRunning}
					<path d="M12 6 L12 12 L16 10" class="spin-indicator" />
				{/if}
				<path d="M12 2 v2" /><path d="M12 20 v2" />
				<path d="M4.93 4.93 l1.41 1.41" /><path d="M17.66 17.66 l1.41 1.41" />
				<path d="M2 12 h2" /><path d="M20 12 h2" />
			</svg>
			{#if isRunning}
				<div class="running-ring" style="border-color: {statusColor}"></div>
			{/if}
		</div>
		<div class="pump-info">
			<div class="pump-name">{pumpName}</div>
			<div class="pump-id">{pumpId}</div>
		</div>
		<div class="pump-status">
			<span class="status-badge" style="background: {statusColor}20; color: {statusColor}; border: 1px solid {statusColor}40;">
				{status.toUpperCase()}
			</span>
		</div>
	</div>

	<div class="pump-metrics">
		<div class="pm-item">
			<span class="pm-label">Flow Rate</span>
			<span class="pm-value" style="color: var(--color-water)">{flowRate.toFixed(1)} <span class="pm-unit">L/min</span></span>
		</div>
		<div class="pm-item">
			<span class="pm-label">Runtime</span>
			<span class="pm-value">{formatRuntime(runtime)}</span>
		</div>
	</div>

	<div class="pump-actions">
		<button
			class="action-btn start"
			on:click={handleStart}
			disabled={disabled || loading || isRunning}
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21" /></svg>
			START
		</button>
		<button
			class="action-btn stop"
			on:click={handleStop}
			disabled={disabled || loading || status === 'stopped'}
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2" /></svg>
			STOP
		</button>
	</div>
</div>

<style>
	.pump-control {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		transition: all 0.2s;
	}
	.pump-control.running {
		border-color: rgba(34, 197, 94, 0.3);
	}
	.pump-control.fault {
		border-color: rgba(239, 68, 68, 0.4);
		animation: pulse 2s infinite;
	}
	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.85; }
	}

	.pump-header {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 14px;
	}
	.pump-icon {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		border: 2px solid;
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		background: var(--bg-secondary);
	}
	.running-ring {
		position: absolute;
		inset: -4px;
		border: 2px solid;
		border-radius: 50%;
		border-top-color: transparent;
		border-right-color: transparent;
		animation: spin 1.5s linear infinite;
	}
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	.spin-indicator {
		animation: spin 2s linear infinite;
		transform-origin: 12px 12px;
	}
	.pump-info {
		flex: 1;
	}
	.pump-name {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
	}
	.pump-id {
		font-size: 0.7rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.status-badge {
		font-size: 0.65rem;
		font-weight: 700;
		padding: 3px 10px;
		border-radius: 4px;
		letter-spacing: 0.05em;
	}

	.pump-metrics {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
		margin-bottom: 14px;
		padding: 10px;
		background: var(--bg-secondary);
		border-radius: 6px;
	}
	.pm-item {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.pm-label {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.pm-value {
		font-family: var(--font-mono);
		font-size: 0.95rem;
		font-weight: 700;
	}
	.pm-unit {
		font-size: 0.65rem;
		color: var(--text-muted);
		font-weight: 400;
	}

	.pump-actions {
		display: flex;
		gap: 8px;
	}
	.action-btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 8px;
		border-radius: 6px;
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		cursor: pointer;
		transition: all 0.15s;
		border: 1px solid;
	}
	.action-btn.start {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
		border-color: rgba(34, 197, 94, 0.3);
	}
	.action-btn.start:hover:not(:disabled) {
		background: rgba(34, 197, 94, 0.3);
	}
	.action-btn.stop {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.3);
	}
	.action-btn.stop:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.3);
	}
	.action-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
</style>
