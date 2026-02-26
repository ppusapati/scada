import type {
	WaterSystem, SolarSystem, Alarm, AlarmCounts,
	Device, SensorReading, AggregatedReading, User
} from '$lib/types/scada';

const API_BASE = '/api/v1';

function getToken(): string | null {
	if (typeof window === 'undefined') return null;
	return localStorage.getItem('scada_token');
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const token = getToken();
	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
		...(options.headers as Record<string, string> || {})
	};
	if (token) {
		headers['Authorization'] = `Bearer ${token}`;
	}

	const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `HTTP ${res.status}`);
	}
	return res.json();
}

// ── Auth ──

export async function login(username: string, password: string): Promise<{ token: string; user: User }> {
	const data = await request<{ token: string; user: User }>('/auth/login', {
		method: 'POST',
		body: JSON.stringify({ username, password })
	});
	localStorage.setItem('scada_token', data.token);
	return data;
}

export async function getMe(): Promise<User> {
	return request<User>('/auth/me');
}

export function logout() {
	localStorage.removeItem('scada_token');
}

// ── Water ──

export async function getWaterSystems(): Promise<WaterSystem[]> {
	return request<WaterSystem[]>('/water/systems');
}

export async function getWaterSystem(id: string): Promise<WaterSystem> {
	return request<WaterSystem>(`/water/systems/${id}`);
}

export async function controlPump(systemId: string, action: 'start' | 'stop'): Promise<void> {
	await request(`/water/systems/${systemId}/pump`, {
		method: 'POST',
		body: JSON.stringify({ action })
	});
}

export async function controlValve(systemId: string, valveId: string, position: number): Promise<void> {
	await request(`/water/systems/${systemId}/valve`, {
		method: 'POST',
		body: JSON.stringify({ valve_id: valveId, position })
	});
}

export async function setWaterMode(systemId: string, mode: string): Promise<void> {
	await request(`/water/systems/${systemId}/mode`, {
		method: 'PUT',
		body: JSON.stringify({ mode })
	});
}

export async function getWaterDevices(): Promise<Device[]> {
	return request<Device[]>('/water/devices');
}

// ── Solar ──

export async function getSolarSystems(): Promise<SolarSystem[]> {
	return request<SolarSystem[]>('/solar/systems');
}

export async function getSolarSystem(id: string): Promise<SolarSystem> {
	return request<SolarSystem>(`/solar/systems/${id}`);
}

export async function controlInverter(systemId: string, action: string): Promise<void> {
	await request(`/solar/systems/${systemId}/inverter`, {
		method: 'POST',
		body: JSON.stringify({ action })
	});
}

export async function setSolarMode(systemId: string, mode: string): Promise<void> {
	await request(`/solar/systems/${systemId}/mode`, {
		method: 'PUT',
		body: JSON.stringify({ mode })
	});
}

export async function getSolarDevices(): Promise<Device[]> {
	return request<Device[]>('/solar/devices');
}

// ── Alarms ──

export async function getActiveAlarms(subsystem?: string): Promise<Alarm[]> {
	const params = subsystem ? `?subsystem=${subsystem}` : '';
	return request<Alarm[]>(`/alarms/active${params}`);
}

export async function getAlarmHistory(params: {
	subsystem?: string; from?: string; to?: string; limit?: number
}): Promise<Alarm[]> {
	const query = new URLSearchParams();
	if (params.subsystem) query.set('subsystem', params.subsystem);
	if (params.from) query.set('from', params.from);
	if (params.to) query.set('to', params.to);
	if (params.limit) query.set('limit', String(params.limit));
	return request<Alarm[]>(`/alarms/history?${query}`);
}

export async function acknowledgeAlarm(id: number): Promise<void> {
	await request(`/alarms/${id}/acknowledge`, { method: 'POST' });
}

export async function clearAlarm(id: number): Promise<void> {
	await request(`/alarms/${id}/clear`, { method: 'POST' });
}

export async function getAlarmCounts(): Promise<AlarmCounts> {
	return request<AlarmCounts>('/alarms/counts');
}

// ── Readings ──

export async function getLatestReading(deviceId: string, metric: string): Promise<SensorReading> {
	return request<SensorReading>(`/readings/device/${deviceId}/latest?metric=${metric}`);
}

export async function getReadingHistory(deviceId: string, params: {
	metric: string; from?: string; to?: string; limit?: number
}): Promise<SensorReading[]> {
	const query = new URLSearchParams({ metric: params.metric });
	if (params.from) query.set('from', params.from);
	if (params.to) query.set('to', params.to);
	if (params.limit) query.set('limit', String(params.limit));
	return request<SensorReading[]>(`/readings/device/${deviceId}/history?${query}`);
}

export async function getAggregatedReadings(deviceId: string, params: {
	metric: string; interval: string; from?: string; to?: string
}): Promise<AggregatedReading[]> {
	const query = new URLSearchParams({ metric: params.metric, interval: params.interval });
	if (params.from) query.set('from', params.from);
	if (params.to) query.set('to', params.to);
	return request<AggregatedReading[]>(`/readings/device/${deviceId}/aggregated?${query}`);
}
