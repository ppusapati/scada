<script>
	import { onMount, onDestroy } from 'svelte';
	import { activeAlarms, alarmCounts, loadAlarms } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { acknowledgeAlarm, clearAlarm, getAlarmHistory } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';

	let filter = 'all'; // all, water, solar
	let showHistory = false;
	let historyAlarms = [];
	let loading = false;

	$: filteredAlarms = filter === 'all'
		? $activeAlarms
		: ($activeAlarms || []).filter(a => a.subsystem === filter);

	let unsubscribe;
	onMount(() => {
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('alarm', () => loadAlarms());
	});
	onDestroy(() => { if (unsubscribe) unsubscribe(); });

	async function handleAck(alarmId) {
		try {
			await acknowledgeAlarm(alarmId);
			loadAlarms();
		} catch (e) {
			console.error('Failed to acknowledge alarm:', e);
		}
	}

	async function handleClear(alarmId) {
		try {
			await clearAlarm(alarmId);
			loadAlarms();
		} catch (e) {
			console.error('Failed to clear alarm:', e);
		}
	}

	async function loadHistory() {
		loading = true;
		try {
			historyAlarms = await getAlarmHistory({ limit: 200 });
			showHistory = true;
		} catch (e) {
			console.error('Failed to load history:', e);
		} finally {
			loading = false;
		}
	}

	function formatTime(ts) {
		return new Date(ts).toLocaleString();
	}
</script>

<div class="alarms-page">
	<header class="page-header">
		<h2>Alarm Management</h2>
		<div class="header-controls">
			<!-- Summary badges -->
			<span class="badge badge-critical">Critical: {$alarmCounts.critical}</span>
			<span class="badge badge-warning">Warning: {$alarmCounts.warning}</span>
			<span class="badge badge-info">Info: {$alarmCounts.info}</span>
		</div>
	</header>

	<!-- Filter Bar -->
	<div class="filter-bar">
		<button class="filter-btn" class:active={filter === 'all'} on:click={() => { filter = 'all'; showHistory = false; }}>
			All Active
		</button>
		<button class="filter-btn" class:active={filter === 'water'} on:click={() => { filter = 'water'; showHistory = false; }}>
			Water
		</button>
		<button class="filter-btn" class:active={filter === 'solar'} on:click={() => { filter = 'solar'; showHistory = false; }}>
			Solar
		</button>
		<div class="filter-spacer"></div>
		<button class="btn" on:click={loadHistory} disabled={loading}>
			{showHistory ? 'Showing History' : 'View History'}
		</button>
	</div>

	<!-- Alarm Table -->
	<div class="alarm-table-container">
		<table class="alarm-table">
			<thead>
				<tr>
					<th>Severity</th>
					<th>Time</th>
					<th>Subsystem</th>
					<th>Category</th>
					<th>Message</th>
					<th>Value / Threshold</th>
					<th>Status</th>
					{#if $isOperator}<th>Actions</th>{/if}
				</tr>
			</thead>
			<tbody>
				{#each (showHistory ? historyAlarms : filteredAlarms) || [] as alarm}
					<tr class="alarm-row" class:critical={alarm.severity === 'critical'} class:warning={alarm.severity === 'warning'}>
						<td>
							<span class="badge" class:badge-critical={alarm.severity === 'critical'}
								class:badge-warning={alarm.severity === 'warning'}
								class:badge-info={alarm.severity === 'info'}>
								{alarm.severity}
							</span>
						</td>
						<td class="mono-text">{formatTime(alarm.occurred_at)}</td>
						<td><span class="subsystem-tag" class:water={alarm.subsystem === 'water'} class:solar={alarm.subsystem === 'solar'}>{alarm.subsystem}</span></td>
						<td>{alarm.category}</td>
						<td class="alarm-message">{alarm.message}</td>
						<td class="mono-text">
							{#if alarm.value !== undefined}
								{alarm.value.toFixed(2)} / {alarm.threshold.toFixed(2)}
							{:else}
								-
							{/if}
						</td>
						<td>
							<span class="badge" class:badge-critical={alarm.status === 'active'}
								class:badge-warning={alarm.status === 'acknowledged'}
								class:badge-success={alarm.status === 'cleared'}>
								{alarm.status}
							</span>
						</td>
						{#if $isOperator}
							<td class="action-cell">
								{#if alarm.status === 'active'}
									<button class="btn-sm" on:click={() => handleAck(alarm.id)}>ACK</button>
								{/if}
								{#if alarm.status !== 'cleared'}
									<button class="btn-sm clear" on:click={() => handleClear(alarm.id)}>CLR</button>
								{/if}
							</td>
						{/if}
					</tr>
				{:else}
					<tr>
						<td colspan={$isOperator ? 8 : 7} class="empty-state">
							No {showHistory ? 'historical' : 'active'} alarms
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

<style>
	.alarms-page { max-width: 1400px; margin: 0 auto; }
	.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
	.page-header h2 { color: var(--color-danger); }
	.header-controls { display: flex; gap: 8px; }

	.filter-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 16px;
		padding: 8px;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}
	.filter-btn {
		padding: 6px 14px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.8rem;
		cursor: pointer;
	}
	.filter-btn.active { background: var(--color-info); color: white; border-color: var(--color-info); }
	.filter-spacer { flex: 1; }

	.alarm-table-container {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: auto;
		max-height: calc(100vh - 240px);
	}

	.alarm-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
	.alarm-table th {
		position: sticky; top: 0;
		background: var(--bg-secondary);
		padding: 10px 12px;
		text-align: left;
		font-size: 0.7rem;
		text-transform: uppercase;
		color: var(--text-muted);
		letter-spacing: 0.05em;
		border-bottom: 1px solid var(--border-color);
	}
	.alarm-table td { padding: 10px 12px; border-bottom: 1px solid var(--border-color); }

	.alarm-row.critical { border-left: 3px solid var(--color-danger); }
	.alarm-row.warning { border-left: 3px solid var(--color-warning); }

	.mono-text { font-family: var(--font-mono); font-size: 0.75rem; }
	.alarm-message { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.subsystem-tag { font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
	.subsystem-tag.water { color: var(--color-water); }
	.subsystem-tag.solar { color: var(--color-solar); }

	.action-cell { display: flex; gap: 4px; }
	.btn-sm {
		padding: 3px 8px;
		font-size: 0.65rem;
		font-weight: 700;
		border: 1px solid var(--border-color);
		border-radius: 3px;
		background: var(--bg-secondary);
		color: var(--text-primary);
		cursor: pointer;
	}
	.btn-sm.clear { color: var(--color-success); border-color: var(--color-success); }
	.btn-sm:hover { background: var(--bg-card-hover); }

	.empty-state { text-align: center; padding: 40px; color: var(--text-muted); }
</style>
