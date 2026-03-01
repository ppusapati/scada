<script>
	export let level = 0;
	export let label = 'Tank';
	export let capacity = 50000;
	export let showCapacity = false;

	$: clampedLevel = Math.max(0, Math.min(100, level));
	$: levelColor = clampedLevel < 15 ? '#ef4444' :
		clampedLevel < 30 ? '#f59e0b' : '#06b6d4';
	$: levelGlow = clampedLevel < 15 ? 'drop-shadow(0 0 8px rgba(239,68,68,0.4))' :
		clampedLevel < 30 ? 'drop-shadow(0 0 8px rgba(245,158,11,0.3))' : 'drop-shadow(0 0 6px rgba(6,182,212,0.3))';

	// SVG dimensions
	const w = 120;
	const h = 180;
	const tankX = 15;
	const tankY = 10;
	const tankW = 90;
	const tankH = 140;
	const wallR = 12;
</script>

<div class="tank-gauge">
	<div class="tank-label">{label}</div>
	<svg width={w} height={h} viewBox="0 0 {w} {h}" style="filter: {levelGlow}">
		<!-- Tank body outline -->
		<rect
			x={tankX} y={tankY}
			width={tankW} height={tankH}
			rx={wallR} ry={wallR}
			fill="none"
			stroke="var(--border-color)"
			stroke-width="2"
		/>

		<!-- Tank inner background -->
		<rect
			x={tankX + 2} y={tankY + 2}
			width={tankW - 4} height={tankH - 4}
			rx={wallR - 1} ry={wallR - 1}
			fill="var(--bg-secondary)"
		/>

		<!-- Water level (clip to tank shape) -->
		<defs>
			<clipPath id="tankClip">
				<rect
					x={tankX + 2} y={tankY + 2}
					width={tankW - 4} height={tankH - 4}
					rx={wallR - 1} ry={wallR - 1}
				/>
			</clipPath>
		</defs>

		<g clip-path="url(#tankClip)">
			<!-- Water body -->
			<rect
				x={tankX + 2}
				y={tankY + 2 + (tankH - 4) * (1 - clampedLevel / 100)}
				width={tankW - 4}
				height={(tankH - 4) * (clampedLevel / 100)}
				fill={levelColor}
				opacity="0.65"
			>
				<animate attributeName="opacity" values="0.55;0.7;0.55" dur="3s" repeatCount="indefinite" />
			</rect>

			<!-- Water surface highlight -->
			{#if clampedLevel > 2}
				<rect
					x={tankX + 2}
					y={tankY + 2 + (tankH - 4) * (1 - clampedLevel / 100) - 1}
					width={tankW - 4}
					height="3"
					fill="white"
					opacity="0.25"
				>
					<animate attributeName="opacity" values="0.15;0.3;0.15" dur="2s" repeatCount="indefinite" />
				</rect>
			{/if}

			<!-- Wave effect -->
			{#if clampedLevel > 3}
				<path
					d="M {tankX+2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)}
					   Q {tankX + tankW*0.25} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) - 3},
					     {tankX + tankW*0.5} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)}
					   Q {tankX + tankW*0.75} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) + 3},
					     {tankX + tankW - 2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)}"
					fill="none"
					stroke="rgba(255,255,255,0.2)"
					stroke-width="2"
				>
					<animate attributeName="d"
						values="M {tankX+2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.25} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) - 3}, {tankX + tankW*0.5} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.75} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) + 3}, {tankX + tankW - 2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)};
						       M {tankX+2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.25} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) + 3}, {tankX + tankW*0.5} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.75} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) - 3}, {tankX + tankW - 2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)};
						       M {tankX+2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.25} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) - 3}, {tankX + tankW*0.5} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)} Q {tankX + tankW*0.75} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100) + 3}, {tankX + tankW - 2} {tankY + 2 + (tankH-4) * (1 - clampedLevel/100)}"
						dur="3s" repeatCount="indefinite" />
				</path>
			{/if}
		</g>

		<!-- Scale marks -->
		{#each [0, 25, 50, 75, 100] as mark}
			{@const markY = tankY + 2 + (tankH - 4) * (1 - mark / 100)}
			<line x1={tankX + tankW + 4} y1={markY} x2={tankX + tankW + 10} y2={markY} stroke="var(--text-muted)" stroke-width="1" opacity="0.5" />
			<text x={tankX + tankW + 13} y={markY + 3} fill="var(--text-muted)" font-size="8" font-family="var(--font-mono)">{mark}</text>
		{/each}

		<!-- Inlet/outlet pipes -->
		<rect x={0} y={tankY + 15} width={tankX} height="6" rx="2" fill="var(--border-color)" opacity="0.6" />
		<rect x={tankX + tankW} y={tankY + tankH - 25} width={w - tankX - tankW} height="6" rx="2" fill="var(--border-color)" opacity="0.6" />
	</svg>

	<div class="tank-reading">
		<span class="tank-value" style="color: {levelColor}">{clampedLevel.toFixed(1)}</span>
		<span class="tank-percent">%</span>
	</div>
	{#if showCapacity}
		<div class="tank-capacity">{(capacity * clampedLevel / 100).toFixed(0).replace(/\B(?=(\d{3})+(?!\d))/g, ',')} / {capacity.toFixed(0).replace(/\B(?=(\d{3})+(?!\d))/g, ',')} L</div>
	{/if}
</div>

<style>
	.tank-gauge {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
	}

	.tank-label {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.tank-reading {
		display: flex;
		align-items: baseline;
		gap: 2px;
	}

	.tank-value {
		font-family: var(--font-mono);
		font-size: 1.6rem;
		font-weight: 800;
		line-height: 1;
	}

	.tank-percent {
		font-family: var(--font-mono);
		font-size: 0.9rem;
		color: var(--text-muted);
		font-weight: 600;
	}

	.tank-capacity {
		font-size: 0.6rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
</style>
