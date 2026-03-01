<script>
	export let value = 0;
	export let min = 0;
	export let max = 100;
	export let label = '';
	export let unit = '';
	export let size = 160;
	export let color = '#3b82f6';
	export let warningThreshold = -1;
	export let dangerThreshold = -1;
	export let showTicks = true;
	export let decimals = 1;

	const strokeWidth = 12;
	const radius = (size - strokeWidth) / 2;
	const center = size / 2;

	// Arc goes from 135deg to 405deg (270deg sweep)
	const startAngle = 135;
	const sweepAngle = 270;

	$: clampedValue = Math.max(min, Math.min(max, value));
	$: percent = (clampedValue - min) / (max - min || 1);
	$: valueAngle = startAngle + percent * sweepAngle;

	$: activeColor = dangerThreshold >= 0 && clampedValue >= dangerThreshold
		? '#ef4444'
		: warningThreshold >= 0 && clampedValue >= warningThreshold
			? '#f59e0b'
			: color;

	function polarToCartesian(cx, cy, r, angleDeg) {
		const angleRad = (angleDeg * Math.PI) / 180;
		return {
			x: cx + r * Math.cos(angleRad),
			y: cy + r * Math.sin(angleRad)
		};
	}

	function describeArc(cx, cy, r, startAng, endAng) {
		const start = polarToCartesian(cx, cy, r, endAng);
		const end = polarToCartesian(cx, cy, r, startAng);
		const largeArcFlag = endAng - startAng > 180 ? 1 : 0;
		return `M ${start.x} ${start.y} A ${r} ${r} 0 ${largeArcFlag} 0 ${end.x} ${end.y}`;
	}

	$: bgArc = describeArc(center, center, radius, startAngle, startAngle + sweepAngle);
	$: valueArc = percent > 0.001 ? describeArc(center, center, radius, startAngle, valueAngle) : '';

	$: ticks = showTicks ? generateTicks(5) : [];

	function generateTicks(count) {
		const result = [];
		for (let i = 0; i <= count; i++) {
			const frac = i / count;
			const ang = startAngle + frac * sweepAngle;
			const inner = polarToCartesian(center, center, radius - strokeWidth / 2 - 4, ang);
			const outer = polarToCartesian(center, center, radius - strokeWidth / 2 - 10, ang);
			const textPos = polarToCartesian(center, center, radius - strokeWidth / 2 - 20, ang);
			const val = min + frac * (max - min);
			result.push({
				x1: inner.x, y1: inner.y,
				x2: outer.x, y2: outer.y,
				tx: textPos.x, ty: textPos.y,
				label: val >= 1000 ? (val / 1000).toFixed(0) + 'k' : val.toFixed(0)
			});
		}
		return result;
	}

	$: displayValue = clampedValue >= 1000
		? (clampedValue / 1000).toFixed(decimals) + 'k'
		: clampedValue.toFixed(decimals);
</script>

<div class="gauge-container" style="width: {size}px; height: {size}px;">
	<svg width={size} height={size} viewBox="0 0 {size} {size}">
		<!-- Background arc -->
		<path
			d={bgArc}
			fill="none"
			stroke="var(--border-color)"
			stroke-width={strokeWidth}
			stroke-linecap="round"
			opacity="0.4"
		/>

		<!-- Value arc -->
		{#if valueArc}
			<path
				d={valueArc}
				fill="none"
				stroke={activeColor}
				stroke-width={strokeWidth}
				stroke-linecap="round"
				class="value-arc"
			/>
		{/if}

		<!-- Tick marks -->
		{#each ticks as tick}
			<line
				x1={tick.x1} y1={tick.y1}
				x2={tick.x2} y2={tick.y2}
				stroke="var(--text-muted)"
				stroke-width="1"
				opacity="0.6"
			/>
			<text
				x={tick.tx} y={tick.ty}
				text-anchor="middle"
				dominant-baseline="middle"
				fill="var(--text-muted)"
				font-size={size > 140 ? '8' : '7'}
				font-family="var(--font-mono)"
			>{tick.label}</text>
		{/each}

		<!-- Center value -->
		<text
			x={center}
			y={center - 4}
			text-anchor="middle"
			dominant-baseline="middle"
			fill={activeColor}
			font-size={size > 140 ? '24' : '18'}
			font-weight="800"
			font-family="var(--font-mono)"
		>{displayValue}</text>

		<!-- Unit -->
		{#if unit}
			<text
				x={center}
				y={center + (size > 140 ? 16 : 12)}
				text-anchor="middle"
				fill="var(--text-muted)"
				font-size={size > 140 ? '11' : '9'}
			>{unit}</text>
		{/if}
	</svg>

	{#if label}
		<div class="gauge-label">{label}</div>
	{/if}
</div>

<style>
	.gauge-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}
	.gauge-label {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		text-align: center;
	}
	.value-arc {
		transition: stroke 0.3s, d 0.5s;
		filter: drop-shadow(0 0 4px currentColor);
	}
</style>
