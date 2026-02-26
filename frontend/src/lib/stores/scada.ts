import { writable } from 'svelte/store';
import type { WaterSystem, SolarSystem, Alarm, AlarmCounts } from '$lib/types/scada';
import {
	getWaterSystems, getSolarSystems,
	getActiveAlarms, getAlarmCounts
} from '$lib/services/api';

// ── Water Store ──
export const waterSystems = writable<WaterSystem[]>([]);
export const currentWaterSystem = writable<WaterSystem | null>(null);

// ── Solar Store ──
export const solarSystems = writable<SolarSystem[]>([]);
export const currentSolarSystem = writable<SolarSystem | null>(null);

// ── Alarms Store ──
export const activeAlarms = writable<Alarm[]>([]);
export const alarmCounts = writable<AlarmCounts>({ critical: 0, warning: 0, info: 0 });

// ── Data Loading ──

export async function loadWaterData() {
	try {
		const systems = await getWaterSystems();
		waterSystems.set(systems);
		if (systems.length > 0) {
			currentWaterSystem.set(systems[0]);
		}
	} catch (e) {
		console.error('Failed to load water data:', e);
	}
}

export async function loadSolarData() {
	try {
		const systems = await getSolarSystems();
		solarSystems.set(systems);
		if (systems.length > 0) {
			currentSolarSystem.set(systems[0]);
		}
	} catch (e) {
		console.error('Failed to load solar data:', e);
	}
}

export async function loadAlarms() {
	try {
		const [alarms, counts] = await Promise.all([
			getActiveAlarms(),
			getAlarmCounts()
		]);
		activeAlarms.set(alarms || []);
		alarmCounts.set(counts);
	} catch (e) {
		console.error('Failed to load alarms:', e);
	}
}

export async function loadAllData() {
	await Promise.all([loadWaterData(), loadSolarData(), loadAlarms()]);
}
