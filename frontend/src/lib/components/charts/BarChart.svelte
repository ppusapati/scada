<script>
	import { onMount } from 'svelte';

	/** @type {{ label: string; value: number }[]} */
	export let data = [];
	export let width = 600;
	export let height = 200;
	export let color = '#f59e0b';
	export let gradientTo = '#fcd34d';
	export let title = '';
	export let unit = '';
	export let showValues = true;
	export let barRadius = 4;

	let containerEl;
	let actualWidth = width;
	let tooltipVisible = false;
	let tooltipX = 0;
	let tooltipY = 0;
	let tooltipLabel = '';
	let tooltipValue = '';

	const padding = { top: 20, right: 16, bottom: 36, left: 48 };

	$: chartWidth = actualWidth - padding.left - padding.right;
	$: chartHeight = height - padding.top - padding.bottom;

	$: maxVal = data.length > 0 ? Math.max(...data.map(d => d.value), 1) : 100;
	$: paddedMax = maxVal * 1.1;

	$: barGap = Math.max(2, Math.min(8, chartWidth / data.length * 0.15));
	$: barWidth = data.length > 0 ? (chartWidth - barGap * (data.length + 1)) / data.length : 0;

	$: bars = data.map((d, i) => {
		const barH = (d.value / paddedMax) * chartHeight;
		return {
			x: padding.left + barGap + i * (barWidth + barGap),
			y: padding.top + chartHeight - barH,
			w: Math.max(barWidth, 2),
			h: barH,
			value: d.value,
			label: d.label
		};
	});

	$: yTicks = generateYTicks(0, paddedMax, 4);

	function generateYTicks(min, max, count) {
		const step = (max - min) / count;
		const ticks = [];
		for (let i = 0; i <= count; i++) {
			const val = min + step * i;
			ticks.push({
				value: val,
				y: padding.top + chartHeight - ((val - min) / (max - min)) * chartHeight,
				label: val >= 1000 ? (val / 1000).toFixed(1) + 'k' : val.toFixed(val >= 10 ? 0 : 1)
			});
		}
		return ticks;
	}

	function handleBarEnter(bar, e) {
		tooltipX = bar.x + bar.w / 2;
		tooltipY = bar.y - 8;
		tooltipLabel = bar.label;
		tooltipValue = bar.value.toFixed(1);
		tooltipVisible = true;
	}

	function handleBarLeave() {
		tooltipVisible = false;
	}

	onMount(() => {
		if (containerEl) {
			const observer = new ResizeObserver(entries => {
				for (const entry of entries) {
					actualWidth = entry.contentRect.width;
				}
			});
			observer.observe(containerEl);
			return () => observer.disconnect();
		}
	});
</script>

<div class="bar-chart-container" bind:this={containerEl}>
	{#if title}
		<div class="chart-title">{title} {#if unit}<span class="chart-unit">({unit})</span>{/if}</div>
	{/if}
	<svg width={actualWidth} {height} viewBox="0 0 {actualWidth} {height}">
		<defs>
			<linearGradient id="barGrad-{color.replace('#','')}" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0%" stop-color={color} />
				<stop offset="100%" stop-color={gradientTo} />
			</linearGradient>
		</defs>

		<!-- Y axis grid -->
		{#each yTicks as tick}
			<line
				x1={padding.left}
				y1={tick.y}
				x2={actualWidth - padding.right}
				y2={tick.y}
				stroke="var(--border-color)"
				stroke-width="1"
				stroke-dasharray="4,4"
				opacity="0.4"
			/>
			<text
				x={padding.left - 8}
				y={tick.y + 4}
				text-anchor="end"
				fill="var(--text-muted)"
				font-size="10"
				font-family="var(--font-mono)"
			>{tick.label}</text>
		{/each}

		<!-- Bars -->
		{#each bars as bar, i}
			<rect
				x={bar.x}
				y={bar.y}
				width={bar.w}
				height={bar.h}
				rx={barRadius}
				ry={barRadius}
				fill="url(#barGrad-{color.replace('#','')})"
				opacity="0.85"
				class="bar-rect"
				on:mouseenter={(e) => handleBarEnter(bar, e)}
				on:mouseleave={handleBarLeave}
			/>
			<!-- X label -->
			{#if data.length <= 31}
				<text
					x={bar.x + bar.w / 2}
					y={height - 6}
					text-anchor="middle"
					fill="var(--text-muted)"
					font-size={data.length > 20 ? '7' : '9'}
					font-family="var(--font-mono)"
					transform={data.length > 12 ? `rotate(-45, ${bar.x + bar.w / 2}, ${height - 6})` : ''}
				>{bar.label}</text>
			{/if}
			<!-- Value on top -->
			{#if showValues && barWidth > 20}
				<text
					x={bar.x + bar.w / 2}
					y={bar.y - 4}
					text-anchor="middle"
					fill={color}
					font-size="9"
					font-weight="600"
					font-family="var(--font-mono)"
				>{bar.value.toFixed(1)}</text>
			{/if}
		{/each}

		<!-- No data message -->
		{#if data.length === 0}
			<text
				x={actualWidth / 2}
				y={height / 2}
				text-anchor="middle"
				fill="var(--text-muted)"
				font-size="13"
			>No data available</text>
		{/if}
	</svg>

	{#if tooltipVisible}
		<div class="tooltip" style="left: {tooltipX}px; top: {tooltipY - 36}px;">
			<div class="tooltip-value" style="color: {color}">{tooltipValue} {unit}</div>
			<div class="tooltip-label">{tooltipLabel}</div>
		</div>
	{/if}
</div>

<style>
	.bar-chart-container {
		position: relative;
		width: 100%;
	}
	.chart-title {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 4px;
	}
	.chart-unit {
		font-weight: 400;
		color: var(--text-muted);
	}
	.bar-rect {
		transition: opacity 0.15s;
		cursor: pointer;
	}
	.bar-rect:hover {
		opacity: 1 !important;
	}
	.tooltip {
		position: absolute;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		padding: 6px 10px;
		pointer-events: none;
		transform: translateX(-50%);
		white-space: nowrap;
		z-index: 10;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}
	.tooltip-value {
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 700;
	}
	.tooltip-label {
		font-size: 0.65rem;
		color: var(--text-muted);
	}
</style>
