<script>
	import { createEventDispatcher } from 'svelte';
	import AlarmBadge from './AlarmBadge.svelte';

	/** @type {import('$lib/types/scada').Alarm[]} */
	export let alarms = [];
	export let showActions = true;
	export let maxHeight = 'calc(100vh - 320px)';
	export let sortable = true;
	export let compact = false;

	const dispatch = createEventDispatcher();

	let sortField = 'occurred_at';
	let sortDir = 'desc';

	$: sortedAlarms = sortAlarms(alarms, sortField, sortDir);

	function sortAlarms(list, field, dir) {
		if (!sortable || !list) return list || [];
		const sorted = [...list].sort((a, b) => {
			let va = a[field];
			let vb = b[field];
			if (field === 'occurred_at' || field === 'acked_at' || field === 'cleared_at') {
				va = new Date(va || 0).getTime();
				vb = new Date(vb || 0).getTime();
			}
			if (field === 'severity') {
				const order = { critical: 3, warning: 2, info: 1 };
				va = order[va] || 0;
				vb = order[vb] || 0;
			}
			if (va < vb) return dir === 'asc' ? -1 : 1;
			if (va > vb) return dir === 'asc' ? 1 : -1;
			return 0;
		});
		return sorted;
	}

	function toggleSort(field) {
		if (!sortable) return;
		if (sortField === field) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortField = field;
			sortDir = 'desc';
		}
	}

	function formatTime(ts) {
		if (!ts) return '--';
		const d = new Date(ts);
		return d.toLocaleString([], {
			month: 'short', day: 'numeric',
			hour: '2-digit', minute: '2-digit', second: '2-digit'
		});
	}

	function handleAck(id) { dispatch('acknowledge', { id }); }
	function handleClear(id) { dispatch('clear', { id }); }

	function getRowClass(alarm) {
		if (alarm.severity === 'critical' && alarm.status === 'active') return 'row-critical';
		if (alarm.severity === 'warning') return 'row-warning';
		return '';
	}
</script>

<div class="alarm-table-wrapper" style="max-height: {maxHeight}">
	<table class="alarm-table" class:compact>
		<thead>
			<tr>
				<th class="th-sort" on:click={() => toggleSort('severity')}>
					Severity {sortField === 'severity' ? (sortDir === 'asc' ? '&#9650;' : '&#9660;') : ''}
				</th>
				<th class="th-sort" on:click={() => toggleSort('occurred_at')}>
					Time {sortField === 'occurred_at' ? (sortDir === 'asc' ? '&#9650;' : '&#9660;') : ''}
				</th>
				<th>Subsystem</th>
				<th>Category</th>
				<th class="col-message">Message</th>
				<th>Value / Limit</th>
				<th class="th-sort" on:click={() => toggleSort('status')}>
					Status {sortField === 'status' ? (sortDir === 'asc' ? '&#9650;' : '&#9660;') : ''}
				</th>
				{#if showActions}
					<th>Actions</th>
				{/if}
			</tr>
		</thead>
		<tbody>
			{#each sortedAlarms as alarm (alarm.id)}
				<tr class={getRowClass(alarm)}>
					<td>
						<AlarmBadge severity={alarm.severity} compact={compact} />
					</td>
					<td class="cell-mono">{formatTime(alarm.occurred_at)}</td>
					<td>
						<span class="subsystem" class:water={alarm.subsystem === 'water'} class:solar={alarm.subsystem === 'solar'}>
							{alarm.subsystem}
						</span>
					</td>
					<td class="cell-category">{alarm.category}</td>
					<td class="cell-message">{alarm.message}</td>
					<td class="cell-mono">
						{#if alarm.value !== undefined && alarm.value !== null}
							{alarm.value.toFixed(2)} / {alarm.threshold.toFixed(2)}
						{:else}
							--
						{/if}
					</td>
					<td>
						<span class="status-tag" class:active={alarm.status === 'active'}
							class:acked={alarm.status === 'acknowledged'}
							class:cleared={alarm.status === 'cleared'}>
							{alarm.status}
						</span>
					</td>
					{#if showActions}
						<td class="cell-actions">
							{#if alarm.status === 'active'}
								<button class="act-btn ack" on:click={() => handleAck(alarm.id)} title="Acknowledge">
									ACK
								</button>
							{/if}
							{#if alarm.status !== 'cleared'}
								<button class="act-btn clr" on:click={() => handleClear(alarm.id)} title="Clear">
									CLR
								</button>
							{/if}
						</td>
					{/if}
				</tr>
			{:else}
				<tr>
					<td colspan={showActions ? 8 : 7} class="empty-state">
						No alarms to display
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.alarm-table-wrapper {
		overflow: auto;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		background: var(--bg-card);
	}
	.alarm-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	.alarm-table.compact {
		font-size: 0.75rem;
	}
	.alarm-table th {
		position: sticky;
		top: 0;
		background: var(--bg-secondary);
		padding: 10px 12px;
		text-align: left;
		font-size: 0.68rem;
		text-transform: uppercase;
		color: var(--text-muted);
		letter-spacing: 0.04em;
		border-bottom: 1px solid var(--border-color);
		white-space: nowrap;
		z-index: 1;
	}
	.th-sort {
		cursor: pointer;
		user-select: none;
	}
	.th-sort:hover {
		color: var(--text-primary);
	}
	.alarm-table td {
		padding: 9px 12px;
		border-bottom: 1px solid var(--border-color);
		vertical-align: middle;
	}
	.alarm-table.compact td {
		padding: 6px 10px;
	}
	.alarm-table tbody tr:hover {
		background: var(--bg-card-hover);
	}

	.row-critical {
		border-left: 3px solid #ef4444;
		background: rgba(239, 68, 68, 0.04);
	}
	.row-warning {
		border-left: 3px solid #f59e0b;
	}

	.cell-mono {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		white-space: nowrap;
	}
	.cell-message {
		max-width: 280px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.col-message {
		min-width: 180px;
	}
	.cell-category {
		font-size: 0.75rem;
	}

	.subsystem {
		font-size: 0.68rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.subsystem.water { color: var(--color-water); }
	.subsystem.solar { color: var(--color-solar); }

	.status-tag {
		font-size: 0.6rem;
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 3px;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.status-tag.active {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}
	.status-tag.acked {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}
	.status-tag.cleared {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.cell-actions {
		display: flex;
		gap: 4px;
		white-space: nowrap;
	}
	.act-btn {
		padding: 3px 8px;
		font-size: 0.6rem;
		font-weight: 700;
		border-radius: 3px;
		cursor: pointer;
		letter-spacing: 0.04em;
		transition: all 0.15s;
		border: 1px solid;
	}
	.act-btn.ack {
		background: rgba(59, 130, 246, 0.12);
		color: #3b82f6;
		border-color: rgba(59, 130, 246, 0.25);
	}
	.act-btn.ack:hover {
		background: rgba(59, 130, 246, 0.25);
	}
	.act-btn.clr {
		background: rgba(34, 197, 94, 0.12);
		color: #22c55e;
		border-color: rgba(34, 197, 94, 0.25);
	}
	.act-btn.clr:hover {
		background: rgba(34, 197, 94, 0.25);
	}

	.empty-state {
		text-align: center;
		padding: 40px 20px;
		color: var(--text-muted);
		font-size: 0.85rem;
	}
</style>
