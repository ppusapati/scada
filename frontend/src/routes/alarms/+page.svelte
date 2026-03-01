<script>
	import { onMount, onDestroy } from 'svelte';
	import { activeAlarms, alarmCounts, loadAlarms } from '$lib/stores/scada';
	import { isOperator } from '$lib/stores/auth';
	import { acknowledgeAlarm, clearAlarm, getAlarmHistory } from '$lib/services/api';
	import { getWebSocket } from '$lib/services/websocket';
	import AlarmTable from '$lib/components/alarms/AlarmTable.svelte';
	import AlarmBadge from '$lib/components/alarms/AlarmBadge.svelte';
	import DateRangePicker from '$lib/components/common/DateRangePicker.svelte';

	let subsystemFilter = 'all';
	let severityFilter = 'all';
	let showHistory = false;
	let historyAlarms = [];
	let loadingHistory = false;
	let historyDateFrom = '';
	let historyDateTo = '';
	let newAlarmFlash = false;
	let audioEnabled = false;

	$: filteredActive = filterAlarms($activeAlarms || [], subsystemFilter, severityFilter);

	function filterAlarms(alarms, subsystem, severity) {
		let result = alarms;
		if (subsystem !== 'all') {
			result = result.filter(a => a.subsystem === subsystem);
		}
		if (severity !== 'all') {
			result = result.filter(a => a.severity === severity);
		}
		return result;
	}

	$: displayAlarms = showHistory ? historyAlarms : filteredActive;

	let unsubscribe;
	let previousAlarmCount = 0;

	onMount(() => {
		previousAlarmCount = ($activeAlarms || []).length;
		const ws = getWebSocket();
		unsubscribe = ws.subscribe('alarm', (msg) => {
			loadAlarms();
			// Flash on new alarm
			const currentCount = ($activeAlarms || []).length;
			if (currentCount > previousAlarmCount) {
				triggerNewAlarmAlert();
			}
			previousAlarmCount = currentCount;
		});
	});

	onDestroy(() => {
		if (unsubscribe) unsubscribe();
	});

	function triggerNewAlarmAlert() {
		newAlarmFlash = true;
		setTimeout(() => { newAlarmFlash = false; }, 2000);
		if (audioEnabled && typeof window !== 'undefined') {
			try {
				const ctx = new AudioContext();
				const osc = ctx.createOscillator();
				const gain = ctx.createGain();
				osc.type = 'sine';
				osc.frequency.setValueAtTime(880, ctx.currentTime);
				gain.gain.setValueAtTime(0.15, ctx.currentTime);
				gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.5);
				osc.connect(gain);
				gain.connect(ctx.destination);
				osc.start(ctx.currentTime);
				osc.stop(ctx.currentTime + 0.5);
			} catch (e) {
				// Audio not available
			}
		}
	}

	async function handleAck(e) {
		const { id } = e.detail;
		try {
			await acknowledgeAlarm(id);
			loadAlarms();
		} catch (e) {
			console.error('Failed to acknowledge alarm:', e);
		}
	}

	async function handleClear(e) {
		const { id } = e.detail;
		try {
			await clearAlarm(id);
			loadAlarms();
		} catch (e) {
			console.error('Failed to clear alarm:', e);
		}
	}

	async function loadHistory() {
		loadingHistory = true;
		try {
			const params = { limit: 500 };
			if (subsystemFilter !== 'all') params.subsystem = subsystemFilter;
			if (historyDateFrom) params.from = new Date(historyDateFrom).toISOString();
			if (historyDateTo) params.to = new Date(historyDateTo).toISOString();
			historyAlarms = await getAlarmHistory(params);
			showHistory = true;
		} catch (e) {
			console.error('Failed to load alarm history:', e);
			historyAlarms = [];
		} finally {
			loadingHistory = false;
		}
	}

	function handleDateChange(e) {
		historyDateFrom = e.detail.from;
		historyDateTo = e.detail.to;
	}

	function showActive() {
		showHistory = false;
		historyAlarms = [];
	}

	function toggleAudio() {
		audioEnabled = !audioEnabled;
	}

	async function ackAll() {
		const active = ($activeAlarms || []).filter(a => a.status === 'active');
		for (const alarm of active) {
			try { await acknowledgeAlarm(alarm.id); } catch (e) { /* continue */ }
		}
		loadAlarms();
	}
</script>

<div class="alarms-page" class:flash={newAlarmFlash}>
	<header class="page-header">
		<div class="header-title">
			<h2>
				<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:inline-block; vertical-align:middle; margin-right:8px; color:var(--color-danger)">
					<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/>
				</svg>
				Alarm Management
			</h2>
		</div>
		<div class="header-actions">
			<button class="audio-toggle" on:click={toggleAudio} class:active={audioEnabled} title={audioEnabled ? 'Mute alarm sounds' : 'Enable alarm sounds'}>
				{#if audioEnabled}
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/></svg>
				{:else}
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19"/><line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/></svg>
				{/if}
			</button>
			{#if $isOperator}
				<button class="btn btn-sm" on:click={ackAll} disabled={($activeAlarms || []).filter(a => a.status === 'active').length === 0}>
					ACK ALL
				</button>
			{/if}
		</div>
	</header>

	<!-- Alarm Count Summary -->
	<div class="alarm-counts">
		<div class="count-card critical" on:click={() => { severityFilter = severityFilter === 'critical' ? 'all' : 'critical'; showActive(); }} class:selected={severityFilter === 'critical'}>
			<div class="cc-count">{$alarmCounts?.critical || 0}</div>
			<div class="cc-label">Critical</div>
			<div class="cc-dot"></div>
		</div>
		<div class="count-card warning" on:click={() => { severityFilter = severityFilter === 'warning' ? 'all' : 'warning'; showActive(); }} class:selected={severityFilter === 'warning'}>
			<div class="cc-count">{$alarmCounts?.warning || 0}</div>
			<div class="cc-label">Warning</div>
			<div class="cc-dot"></div>
		</div>
		<div class="count-card info" on:click={() => { severityFilter = severityFilter === 'info' ? 'all' : 'info'; showActive(); }} class:selected={severityFilter === 'info'}>
			<div class="cc-count">{$alarmCounts?.info || 0}</div>
			<div class="cc-label">Info</div>
			<div class="cc-dot"></div>
		</div>
		<div class="count-card total">
			<div class="cc-count">{($alarmCounts?.critical || 0) + ($alarmCounts?.warning || 0) + ($alarmCounts?.info || 0)}</div>
			<div class="cc-label">Total Active</div>
		</div>
	</div>

	<!-- Filter & View Controls -->
	<div class="filter-bar">
		<div class="filter-group">
			<span class="filter-label">View:</span>
			<button class="filter-btn" class:active={!showHistory} on:click={showActive}>Active</button>
			<button class="filter-btn" class:active={showHistory} on:click={loadHistory} disabled={loadingHistory}>
				{loadingHistory ? 'Loading...' : 'History'}
			</button>
		</div>

		<div class="filter-group">
			<span class="filter-label">Subsystem:</span>
			<button class="filter-btn" class:active={subsystemFilter === 'all'} on:click={() => subsystemFilter = 'all'}>All</button>
			<button class="filter-btn water" class:active={subsystemFilter === 'water'} on:click={() => subsystemFilter = 'water'}>Water</button>
			<button class="filter-btn solar" class:active={subsystemFilter === 'solar'} on:click={() => subsystemFilter = 'solar'}>Solar</button>
		</div>

		{#if showHistory}
			<div class="filter-group date-group">
				<DateRangePicker on:change={handleDateChange} />
				<button class="btn btn-primary btn-sm" on:click={loadHistory} disabled={loadingHistory}>
					Apply
				</button>
			</div>
		{/if}

		<div class="filter-summary">
			Showing <strong>{displayAlarms.length}</strong> {showHistory ? 'historical' : 'active'} alarm{displayAlarms.length !== 1 ? 's' : ''}
		</div>
	</div>

	<!-- Alarm Table -->
	<AlarmTable
		alarms={displayAlarms}
		showActions={$isOperator && !showHistory}
		on:acknowledge={handleAck}
		on:clear={handleClear}
	/>
</div>

<style>
	.alarms-page {
		max-width: 1400px;
		margin: 0 auto;
		transition: background-color 0.3s;
	}
	.alarms-page.flash {
		animation: alarmFlash 0.5s ease-in-out 2;
	}
	@keyframes alarmFlash {
		0%, 100% { background-color: transparent; }
		50% { background-color: rgba(239, 68, 68, 0.05); }
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 16px;
		flex-wrap: wrap;
		gap: 12px;
	}
	.header-title h2 { color: var(--color-danger); font-size: 1.3rem; display: flex; align-items: center; }
	.header-actions { display: flex; gap: 8px; align-items: center; }

	.audio-toggle {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border-radius: 8px;
		border: 1px solid var(--border-color);
		background: var(--bg-card);
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}
	.audio-toggle.active { color: var(--color-info); border-color: rgba(59,130,246,0.3); background: rgba(59,130,246,0.08); }
	.audio-toggle:hover { background: var(--bg-card-hover); }

	.btn-sm {
		padding: 6px 14px;
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.04em;
	}

	/* Alarm Counts */
	.alarm-counts {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
		margin-bottom: 16px;
	}
	.count-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 16px;
		text-align: center;
		cursor: pointer;
		transition: all 0.15s;
		position: relative;
		overflow: hidden;
	}
	.count-card:hover { background: var(--bg-card-hover); transform: translateY(-1px); }
	.count-card.selected { border-width: 2px; }
	.count-card.critical { border-color: rgba(239,68,68,0.2); }
	.count-card.critical.selected { border-color: #ef4444; background: rgba(239,68,68,0.06); }
	.count-card.warning { border-color: rgba(245,158,11,0.2); }
	.count-card.warning.selected { border-color: #f59e0b; background: rgba(245,158,11,0.06); }
	.count-card.info { border-color: rgba(59,130,246,0.2); }
	.count-card.info.selected { border-color: #3b82f6; background: rgba(59,130,246,0.06); }
	.cc-count { font-family: var(--font-mono); font-size: 2rem; font-weight: 800; line-height: 1; }
	.critical .cc-count { color: #ef4444; }
	.warning .cc-count { color: #f59e0b; }
	.info .cc-count { color: #3b82f6; }
	.total .cc-count { color: var(--text-primary); }
	.cc-label { font-size: 0.65rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-top: 4px; }
	.cc-dot {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
	}
	.critical .cc-dot { background: #ef4444; box-shadow: 0 0 6px #ef4444; animation: pulse 1.5s infinite; }
	.warning .cc-dot { background: #f59e0b; }
	.info .cc-dot { background: #3b82f6; }
	@keyframes pulse { 0%,100% { opacity:1 } 50% { opacity:0.4 } }

	/* Filter Bar */
	.filter-bar {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 10px 16px;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		margin-bottom: 14px;
		flex-wrap: wrap;
	}
	.filter-group {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.filter-label {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		white-space: nowrap;
	}
	.filter-btn {
		padding: 5px 12px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}
	.filter-btn:hover { background: var(--bg-card-hover); }
	.filter-btn.active { background: var(--color-info); color: white; border-color: var(--color-info); }
	.filter-btn.water.active { background: var(--color-water); border-color: var(--color-water); }
	.filter-btn.solar.active { background: var(--color-solar); border-color: var(--color-solar); color: #000; }
	.filter-btn:disabled { opacity: 0.5; cursor: not-allowed; }

	.date-group {
		flex-wrap: wrap;
	}

	.filter-summary {
		margin-left: auto;
		font-size: 0.75rem;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.filter-summary strong {
		color: var(--text-primary);
	}

	/* Responsive */
	@media (max-width: 768px) {
		.alarm-counts { grid-template-columns: repeat(2, 1fr); }
		.filter-bar { flex-direction: column; align-items: flex-start; }
		.filter-summary { margin-left: 0; }
	}
</style>
