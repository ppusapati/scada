<script>
	export let strings = [];
	export let highlightFaults = true;

	// Generate mock string data if none provided
	$: displayStrings = strings.length > 0 ? strings : generateMockStrings();

	function generateMockStrings() {
		const result = [];
		for (let i = 1; i <= 16; i++) {
			const baseVoltage = 38 + Math.random() * 4;
			const baseCurrent = 8 + Math.random() * 2;
			const isFault = i === 7; // simulate one faulted string
			result.push({
				id: i,
				name: `String ${i}`,
				voltage: isFault ? 0 : baseVoltage,
				current: isFault ? 0 : baseCurrent,
				power: isFault ? 0 : baseVoltage * baseCurrent,
				status: isFault ? 'fault' : 'normal',
				temperature: 42 + Math.random() * 8
			});
		}
		return result;
	}

	$: totalPower = displayStrings.reduce((sum, s) => sum + (s.power || s.voltage * s.current), 0);
	$: avgVoltage = displayStrings.filter(s => s.status !== 'fault').length > 0
		? displayStrings.filter(s => s.status !== 'fault').reduce((sum, s) => sum + s.voltage, 0) / displayStrings.filter(s => s.status !== 'fault').length
		: 0;
	$: totalCurrent = displayStrings.reduce((sum, s) => sum + s.current, 0);
</script>

<div class="string-table-container">
	<div class="table-header">
		<h3>String Performance</h3>
		<div class="table-summary">
			<span class="summary-item">
				<span class="summary-label">Total Power</span>
				<span class="summary-value solar-color">{(totalPower / 1000).toFixed(2)} kW</span>
			</span>
			<span class="summary-item">
				<span class="summary-label">Avg Voltage</span>
				<span class="summary-value">{avgVoltage.toFixed(1)} V</span>
			</span>
			<span class="summary-item">
				<span class="summary-label">Total Current</span>
				<span class="summary-value">{totalCurrent.toFixed(1)} A</span>
			</span>
		</div>
	</div>

	<div class="table-scroll">
		<table class="string-table">
			<thead>
				<tr>
					<th>#</th>
					<th>String</th>
					<th>Voltage (V)</th>
					<th>Current (A)</th>
					<th>Power (W)</th>
					<th>Temp (&deg;C)</th>
					<th>Status</th>
				</tr>
			</thead>
			<tbody>
				{#each displayStrings as str}
					<tr class:fault-row={highlightFaults && str.status === 'fault'}>
						<td class="row-num">{str.id}</td>
						<td class="str-name">{str.name}</td>
						<td class="mono-val" class:zero={str.voltage === 0}>{str.voltage.toFixed(1)}</td>
						<td class="mono-val" class:zero={str.current === 0}>{str.current.toFixed(2)}</td>
						<td class="mono-val power-val" class:zero={str.power === 0 && str.voltage * str.current === 0}>
							{(str.power || str.voltage * str.current).toFixed(0)}
						</td>
						<td class="mono-val" class:hot={str.temperature > 50}>{str.temperature?.toFixed(1) ?? '--'}</td>
						<td>
							{#if str.status === 'fault'}
								<span class="status-tag fault">FAULT</span>
							{:else if str.status === 'warning'}
								<span class="status-tag warning">WARN</span>
							{:else}
								<span class="status-tag normal">OK</span>
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
			<tfoot>
				<tr class="total-row">
					<td colspan="2">TOTAL / AVG</td>
					<td class="mono-val">{avgVoltage.toFixed(1)}</td>
					<td class="mono-val">{totalCurrent.toFixed(1)}</td>
					<td class="mono-val power-val">{totalPower.toFixed(0)}</td>
					<td class="mono-val">--</td>
					<td>
						<span class="status-tag normal">{displayStrings.filter(s => s.status !== 'fault').length}/{displayStrings.length}</span>
					</td>
				</tr>
			</tfoot>
		</table>
	</div>
</div>

<style>
	.string-table-container {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}
	.table-header {
		padding: 14px 16px;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}
	.table-header h3 {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--color-solar);
	}
	.table-summary {
		display: flex;
		gap: 16px;
	}
	.summary-item {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.summary-label {
		font-size: 0.6rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.summary-value {
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 700;
	}
	.solar-color { color: var(--color-solar); }

	.table-scroll {
		overflow-x: auto;
	}
	.string-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	.string-table th {
		background: var(--bg-secondary);
		padding: 8px 12px;
		text-align: left;
		font-size: 0.65rem;
		text-transform: uppercase;
		color: var(--text-muted);
		letter-spacing: 0.04em;
		border-bottom: 1px solid var(--border-color);
		position: sticky;
		top: 0;
	}
	.string-table td {
		padding: 7px 12px;
		border-bottom: 1px solid var(--border-color);
	}
	.string-table tbody tr:hover {
		background: var(--bg-card-hover);
	}
	.fault-row {
		background: rgba(239, 68, 68, 0.06) !important;
		border-left: 3px solid #ef4444;
	}
	.row-num {
		font-family: var(--font-mono);
		font-size: 0.7rem;
		color: var(--text-muted);
		width: 30px;
	}
	.str-name {
		font-weight: 500;
	}
	.mono-val {
		font-family: var(--font-mono);
		font-weight: 600;
		color: var(--text-primary);
	}
	.mono-val.zero {
		color: var(--color-danger);
	}
	.mono-val.hot {
		color: var(--color-warning);
	}
	.power-val {
		color: var(--color-solar);
	}
	.status-tag {
		font-size: 0.6rem;
		font-weight: 700;
		padding: 2px 8px;
		border-radius: 3px;
		letter-spacing: 0.04em;
	}
	.status-tag.normal {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}
	.status-tag.fault {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
		animation: pulse 1.5s infinite;
	}
	.status-tag.warning {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}
	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
	.total-row {
		background: var(--bg-secondary);
		font-weight: 700;
	}
	.total-row td {
		border-bottom: none;
		padding: 10px 12px;
		font-size: 0.8rem;
	}
</style>
