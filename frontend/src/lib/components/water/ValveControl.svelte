<script>
	import { createEventDispatcher } from 'svelte';

	export let valveId = 'V-001';
	export let valveName = 'Inlet Valve';
	/** @type {number} position 0-100 */
	export let position = 0;
	export let disabled = false;
	export let loading = false;

	const dispatch = createEventDispatcher();

	$: isOpen = position > 95;
	$: isClosed = position < 5;
	$: isPartial = !isOpen && !isClosed;
	$: statusText = isOpen ? 'OPEN' : isClosed ? 'CLOSED' : `${position.toFixed(0)}%`;
	$: statusColor = isOpen ? '#22c55e' : isClosed ? '#ef4444' : '#f59e0b';

	function handleOpen() { dispatch('command', { valve_id: valveId, position: 100 }); }
	function handleClose() { dispatch('command', { valve_id: valveId, position: 0 }); }
	function handleSetPosition(e) {
		const pos = parseFloat(e.target.value);
		dispatch('command', { valve_id: valveId, position: pos });
	}
</script>

<div class="valve-control" class:open={isOpen} class:closed={isClosed}>
	<div class="valve-header">
		<div class="valve-visual">
			<svg width="36" height="36" viewBox="0 0 40 40">
				<!-- Pipe -->
				<rect x="0" y="15" width="40" height="10" fill="var(--bg-secondary)" stroke="var(--border-color)" stroke-width="1" rx="2"/>
				<!-- Valve body -->
				<rect x="14" y="10" width="12" height="20" rx="3" fill={statusColor + '30'} stroke={statusColor} stroke-width="1.5"/>
				<!-- Flow indicator -->
				{#if position > 5}
					<rect x="3" y="17.5" width={position / 100 * 34} height="5" fill={statusColor} opacity="0.5" rx="1"/>
				{/if}
				<!-- Handle -->
				<line x1="20" y1="4" x2="20" y2="12" stroke={statusColor} stroke-width="2.5" stroke-linecap="round"/>
				<circle cx="20" cy="4" r="3" fill={statusColor} opacity="0.8"/>
			</svg>
		</div>
		<div class="valve-info">
			<div class="valve-name">{valveName}</div>
			<div class="valve-id">{valveId}</div>
		</div>
		<div class="valve-status" style="color: {statusColor}">
			{statusText}
		</div>
	</div>

	<!-- Position slider -->
	<div class="valve-slider-container">
		<div class="slider-track">
			<div class="slider-fill" style="width: {position}%; background: {statusColor}"></div>
		</div>
		<input
			type="range"
			min="0"
			max="100"
			step="5"
			value={position}
			on:change={handleSetPosition}
			disabled={disabled || loading}
			class="valve-slider"
		/>
		<div class="slider-labels">
			<span>0%</span>
			<span>50%</span>
			<span>100%</span>
		</div>
	</div>

	<div class="valve-actions">
		<button
			class="v-btn open-btn"
			on:click={handleOpen}
			disabled={disabled || loading || isOpen}
		>OPEN</button>
		<button
			class="v-btn close-btn"
			on:click={handleClose}
			disabled={disabled || loading || isClosed}
		>CLOSE</button>
	</div>
</div>

<style>
	.valve-control {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 14px;
		transition: all 0.2s;
	}
	.valve-control.open {
		border-color: rgba(34, 197, 94, 0.25);
	}
	.valve-control.closed {
		border-color: rgba(239, 68, 68, 0.2);
	}

	.valve-header {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 12px;
	}
	.valve-visual {
		flex-shrink: 0;
	}
	.valve-info {
		flex: 1;
	}
	.valve-name {
		font-size: 0.85rem;
		font-weight: 600;
	}
	.valve-id {
		font-size: 0.65rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.valve-status {
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 700;
	}

	.valve-slider-container {
		position: relative;
		margin-bottom: 12px;
	}
	.slider-track {
		height: 6px;
		background: var(--bg-secondary);
		border-radius: 3px;
		overflow: hidden;
		margin-bottom: 4px;
	}
	.slider-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}
	.valve-slider {
		width: 100%;
		-webkit-appearance: none;
		appearance: none;
		height: 6px;
		background: transparent;
		outline: none;
		position: absolute;
		top: 0;
		left: 0;
		margin: 0;
		cursor: pointer;
	}
	.valve-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--text-primary);
		border: 2px solid var(--bg-card);
		cursor: pointer;
		box-shadow: 0 0 4px rgba(0,0,0,0.3);
	}
	.valve-slider:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.slider-labels {
		display: flex;
		justify-content: space-between;
		font-size: 0.6rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
		margin-top: 4px;
	}

	.valve-actions {
		display: flex;
		gap: 6px;
	}
	.v-btn {
		flex: 1;
		padding: 6px;
		border-radius: 4px;
		font-size: 0.65rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		cursor: pointer;
		transition: all 0.15s;
		border: 1px solid;
	}
	.open-btn {
		background: rgba(34, 197, 94, 0.12);
		color: #22c55e;
		border-color: rgba(34, 197, 94, 0.25);
	}
	.open-btn:hover:not(:disabled) {
		background: rgba(34, 197, 94, 0.25);
	}
	.close-btn {
		background: rgba(239, 68, 68, 0.12);
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.25);
	}
	.close-btn:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.25);
	}
	.v-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
</style>
