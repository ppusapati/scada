<script>
	import { onMount, afterUpdate } from 'svelte';

	/** @type {{ timestamp: string; value: number }[]} */
	export let data = [];
	export let width = 600;
	export let height = 200;
	export let color = '#3b82f6';
	export let fillColor = '';
	export let label = '';
	export let unit = '';
	export let showGrid = true;
	export let showDots = false;
	export let showArea = true;
	export let showTooltip = true;
	export let yMin = undefined;
	export let yMax = undefined;
	export let animate = true;

	let svgEl;
	let containerEl;
	let actualWidth = width;
	let tooltipVisible = false;
	let tooltipX = 0;
	let tooltipY = 0;
	let tooltipValue = '';
	let tooltipTime = '';

	const padding = { top: 20, right: 16, bottom: 32, left: 56 };

	$: chartWidth = actualWidth - padding.left - padding.right;
	$: chartHeight = height - padding.top - padding.bottom;

	$: values = data.map(d => d.value);
	$: computedMin = yMin !== undefined ? yMin : (values.length > 0 ? Math.min(...values) : 0);
	$: computedMax = yMax !== undefined ? yMax : (values.length > 0 ? Math.max(...values) : 100);
	$: range = computedMax - computedMin || 1;
	$: paddedMin = computedMin - range * 0.05;
	$: paddedMax = computedMax + range * 0.05;
	$: paddedRange = paddedMax - paddedMin || 1;

	$: points = data.map((d, i) => ({
		x: padding.left + (data.length > 1 ? (i / (data.length - 1)) * chartWidth : chartWidth / 2),
		y: padding.top + chartHeight - ((d.value - paddedMin) / paddedRange) * chartHeight,
		value: d.value,
		timestamp: d.timestamp
	}));

	$: pathD = points.length > 1
		? 'M ' + points.map(p => `${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' L ')
		: '';

	$: areaD = points.length > 1
		? pathD + ` L ${points[points.length - 1].x.toFixed(2)},${padding.top + chartHeight} L ${points[0].x.toFixed(2)},${padding.top + chartHeight} Z`
		: '';

	$: yTicks = generateYTicks(paddedMin, paddedMax, 5);
	$: xTicks = generateXTicks(data, 5);

	$: computedFillColor = fillColor || color + '20';

	function generateYTicks(min, max, count) {
		const step = (max - min) / count;
		const ticks = [];
		for (let i = 0; i <= count; i++) {
			const val = min + step * i;
			ticks.push({
				value: val,
				y: padding.top + chartHeight - ((val - paddedMin) / paddedRange) * chartHeight,
				label: val.toFixed(val >= 100 ? 0 : val >= 10 ? 1 : 2)
			});
		}
		return ticks;
	}

	function generateXTicks(data, count) {
		if (data.length < 2) return [];
		const ticks = [];
		const step = Math.max(1, Math.floor(data.length / count));
		for (let i = 0; i < data.length; i += step) {
			const d = data[i];
			const x = padding.left + (i / (data.length - 1)) * chartWidth;
			const dt = new Date(d.timestamp);
			ticks.push({
				x,
				label: dt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
			});
		}
		return ticks;
	}

	function handleMouseMove(e) {
		if (!showTooltip || points.length === 0) return;
		const rect = svgEl.getBoundingClientRect();
		const mouseX = e.clientX - rect.left;
		let closest = points[0];
		let minDist = Infinity;
		for (const p of points) {
			const dist = Math.abs(p.x - mouseX);
			if (dist < minDist) {
				minDist = dist;
				closest = p;
			}
		}
		tooltipX = closest.x;
		tooltipY = closest.y;
		tooltipValue = closest.value.toFixed(2);
		tooltipTime = new Date(closest.timestamp).toLocaleString();
		tooltipVisible = true;
	}

	function handleMouseLeave() {
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

<div class="line-chart-container" bind:this={containerEl}>
	{#if label}
		<div class="chart-label">{label} {#if unit}<span class="chart-unit">({unit})</span>{/if}</div>
	{/if}
	<svg
		bind:this={svgEl}
		width={actualWidth}
		{height}
		viewBox="0 0 {actualWidth} {height}"
		on:mousemove={handleMouseMove}
		on:mouseleave={handleMouseLeave}
		class="line-chart-svg"
	>
		<!-- Grid lines -->
		{#if showGrid}
			{#each yTicks as tick}
				<line
					x1={padding.left}
					y1={tick.y}
					x2={actualWidth - padding.right}
					y2={tick.y}
					stroke="var(--border-color)"
					stroke-width="1"
					stroke-dasharray="4,4"
					opacity="0.5"
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
			{#each xTicks as tick}
				<text
					x={tick.x}
					y={height - 6}
					text-anchor="middle"
					fill="var(--text-muted)"
					font-size="9"
					font-family="var(--font-mono)"
				>{tick.label}</text>
			{/each}
		{/if}

		<!-- Area fill -->
		{#if showArea && areaD}
			<path d={areaD} fill={computedFillColor} />
		{/if}

		<!-- Line -->
		{#if pathD}
			<path
				d={pathD}
				fill="none"
				stroke={color}
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				class:animate-line={animate}
			/>
		{/if}

		<!-- Data dots -->
		{#if showDots}
			{#each points as point}
				<circle cx={point.x} cy={point.y} r="3" fill={color} stroke="var(--bg-card)" stroke-width="1.5" />
			{/each}
		{/if}

		<!-- Tooltip crosshair -->
		{#if tooltipVisible}
			<line x1={tooltipX} y1={padding.top} x2={tooltipX} y2={padding.top + chartHeight} stroke={color} stroke-width="1" stroke-dasharray="3,3" opacity="0.6" />
			<circle cx={tooltipX} cy={tooltipY} r="5" fill={color} stroke="var(--bg-card)" stroke-width="2" />
		{/if}

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

	<!-- Tooltip box -->
	{#if tooltipVisible}
		<div class="tooltip" style="left: {tooltipX}px; top: {tooltipY - 48}px;">
			<div class="tooltip-value" style="color: {color}">{tooltipValue} {unit}</div>
			<div class="tooltip-time">{tooltipTime}</div>
		</div>
	{/if}
</div>

<style>
	.line-chart-container {
		position: relative;
		width: 100%;
	}
	.chart-label {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-weight: 600;
		margin-bottom: 4px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.chart-unit {
		font-weight: 400;
		color: var(--text-muted);
	}
	.line-chart-svg {
		display: block;
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
	.tooltip-time {
		font-size: 0.65rem;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}
	.animate-line {
		animation: drawLine 1s ease-out;
	}
	@keyframes drawLine {
		from { stroke-dasharray: 2000; stroke-dashoffset: 2000; }
		to { stroke-dasharray: 2000; stroke-dashoffset: 0; }
	}
</style>
