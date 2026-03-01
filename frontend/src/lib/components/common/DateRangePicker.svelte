<script>
	import { createEventDispatcher } from 'svelte';

	export let from = '';
	export let to = '';
	export let presets = true;

	const dispatch = createEventDispatcher();

	// Initialize defaults
	if (!from) {
		const d = new Date();
		d.setDate(d.getDate() - 1);
		from = d.toISOString().slice(0, 16);
	}
	if (!to) {
		to = new Date().toISOString().slice(0, 16);
	}

	const presetOptions = [
		{ label: 'Last 1H', hours: 1 },
		{ label: 'Last 6H', hours: 6 },
		{ label: 'Last 24H', hours: 24 },
		{ label: 'Last 7D', hours: 168 },
		{ label: 'Last 30D', hours: 720 },
	];

	function applyPreset(hours) {
		const now = new Date();
		const start = new Date(now.getTime() - hours * 60 * 60 * 1000);
		from = start.toISOString().slice(0, 16);
		to = now.toISOString().slice(0, 16);
		emitChange();
	}

	function emitChange() {
		dispatch('change', {
			from: new Date(from).toISOString(),
			to: new Date(to).toISOString()
		});
	}

	function handleFromChange(e) {
		from = e.target.value;
		emitChange();
	}

	function handleToChange(e) {
		to = e.target.value;
		emitChange();
	}
</script>

<div class="date-range-picker">
	{#if presets}
		<div class="presets">
			{#each presetOptions as preset}
				<button class="preset-btn" on:click={() => applyPreset(preset.hours)}>
					{preset.label}
				</button>
			{/each}
		</div>
	{/if}
	<div class="inputs">
		<div class="field">
			<label>From</label>
			<input type="datetime-local" value={from} on:change={handleFromChange} />
		</div>
		<span class="separator">-</span>
		<div class="field">
			<label>To</label>
			<input type="datetime-local" value={to} on:change={handleToChange} />
		</div>
	</div>
</div>

<style>
	.date-range-picker {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.presets {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.preset-btn {
		padding: 4px 10px;
		font-size: 0.7rem;
		font-weight: 600;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: var(--bg-secondary);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.15s;
	}
	.preset-btn:hover {
		background: var(--color-info);
		color: white;
		border-color: var(--color-info);
	}
	.inputs {
		display: flex;
		align-items: flex-end;
		gap: 8px;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.field label {
		font-size: 0.65rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.field input {
		background: var(--bg-secondary);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 0.8rem;
		font-family: var(--font-mono);
	}
	.field input:focus {
		outline: none;
		border-color: var(--color-info);
	}
	.separator {
		color: var(--text-muted);
		padding-bottom: 6px;
	}
</style>
