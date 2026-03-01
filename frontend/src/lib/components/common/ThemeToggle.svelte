<script>
	import { onMount } from 'svelte';

	let isDark = true;

	onMount(() => {
		const saved = localStorage.getItem('scada_theme');
		if (saved === 'light') {
			isDark = false;
			applyTheme(false);
		}
	});

	function toggle() {
		isDark = !isDark;
		applyTheme(isDark);
		localStorage.setItem('scada_theme', isDark ? 'dark' : 'light');
	}

	function applyTheme(dark) {
		const root = document.documentElement;
		if (dark) {
			root.style.setProperty('--bg-primary', '#0a0e17');
			root.style.setProperty('--bg-secondary', '#111827');
			root.style.setProperty('--bg-card', '#1a2332');
			root.style.setProperty('--bg-card-hover', '#1f2b3d');
			root.style.setProperty('--border-color', '#2a3a4e');
			root.style.setProperty('--text-primary', '#e5e7eb');
			root.style.setProperty('--text-secondary', '#9ca3af');
			root.style.setProperty('--text-muted', '#6b7280');
			document.body.classList.remove('light-theme');
		} else {
			root.style.setProperty('--bg-primary', '#f3f4f6');
			root.style.setProperty('--bg-secondary', '#ffffff');
			root.style.setProperty('--bg-card', '#ffffff');
			root.style.setProperty('--bg-card-hover', '#f9fafb');
			root.style.setProperty('--border-color', '#e5e7eb');
			root.style.setProperty('--text-primary', '#111827');
			root.style.setProperty('--text-secondary', '#4b5563');
			root.style.setProperty('--text-muted', '#9ca3af');
			document.body.classList.add('light-theme');
		}
	}
</script>

<button class="theme-toggle" on:click={toggle} title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}>
	{#if isDark}
		<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
			<circle cx="12" cy="12" r="5"/>
			<line x1="12" y1="1" x2="12" y2="3"/>
			<line x1="12" y1="21" x2="12" y2="23"/>
			<line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
			<line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
			<line x1="1" y1="12" x2="3" y2="12"/>
			<line x1="21" y1="12" x2="23" y2="12"/>
			<line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
			<line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
		</svg>
	{:else}
		<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
			<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
		</svg>
	{/if}
</button>

<style>
	.theme-toggle {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border-radius: 8px;
		border: 1px solid var(--border-color);
		background: var(--bg-card);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	.theme-toggle:hover {
		background: var(--bg-card-hover);
		color: var(--text-primary);
	}
</style>
