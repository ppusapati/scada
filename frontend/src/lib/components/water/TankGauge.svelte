<script>
	export let level = 0;
	export let label = 'Tank';

	$: clampedLevel = Math.max(0, Math.min(100, level));
	$: levelColor = clampedLevel < 20 ? 'var(--color-danger)' :
		clampedLevel < 40 ? 'var(--color-warning)' : 'var(--color-water)';
</script>

<div class="tank-gauge">
	<div class="tank-label">{label}</div>
	<div class="tank-container">
		<div class="tank-body">
			<div class="water-level" style="height: {clampedLevel}%; background: {levelColor}">
				<div class="water-surface"></div>
			</div>
			<!-- Scale marks -->
			{#each [0, 25, 50, 75, 100] as mark}
				<div class="scale-mark" style="bottom: {mark}%">
					<span class="scale-label">{mark}%</span>
				</div>
			{/each}
		</div>
	</div>
	<div class="tank-value" style="color: {levelColor}">
		{clampedLevel.toFixed(1)}%
	</div>
</div>

<style>
	.tank-gauge {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
	}

	.tank-label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.tank-container {
		width: 120px;
		height: 200px;
	}

	.tank-body {
		width: 100%;
		height: 100%;
		background: var(--bg-secondary);
		border: 2px solid var(--border-color);
		border-radius: 0 0 12px 12px;
		position: relative;
		overflow: hidden;
	}

	.water-level {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		transition: height 1s ease, background 0.5s;
		opacity: 0.8;
	}

	.water-surface {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 4px;
		background: rgba(255, 255, 255, 0.3);
		animation: wave 2s ease-in-out infinite;
	}

	@keyframes wave {
		0%, 100% { transform: translateX(-2px); }
		50% { transform: translateX(2px); }
	}

	.scale-mark {
		position: absolute;
		left: 0;
		right: 0;
		border-top: 1px dashed var(--border-color);
	}

	.scale-label {
		position: absolute;
		right: -40px;
		top: -8px;
		font-size: 0.6rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.tank-value {
		font-family: var(--font-mono);
		font-size: 1.4rem;
		font-weight: 700;
	}
</style>
