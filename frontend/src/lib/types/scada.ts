// ── Core Types ──

export interface Device {
	id: string;
	name: string;
	device_type: string;
	location: string;
	subsystem: 'water' | 'solar';
	status: 'online' | 'offline' | 'fault' | 'maintenance';
	last_seen: string;
	metadata: Record<string, unknown>;
}

export interface SensorReading {
	id: number;
	device_id: string;
	timestamp: string;
	metric: string;
	value: number;
	unit: string;
	quality: number;
}

// ── Water System ──

export interface WaterSystem {
	id: string;
	name: string;
	tank_level: number;
	flow_rate_in: number;
	flow_rate_out: number;
	pressure: number;
	ph_level: number;
	turbidity: number;
	chlorine_level: number;
	pump_status: 'running' | 'stopped' | 'fault';
	valve_positions: Record<string, number>;
	operating_mode: 'auto' | 'manual' | 'maintenance';
	updated_at: string;
}

// ── Solar System ──

export interface SolarSystem {
	id: string;
	name: string;
	total_capacity_kw: number;
	current_output_kw: number;
	irradiance: number;
	panel_temperature: number;
	inverter_status: string;
	grid_voltage: number;
	grid_frequency: number;
	daily_energy_kwh: number;
	total_energy_mwh: number;
	efficiency: number;
	operating_mode: string;
	updated_at: string;
}

// ── Alarms ──

export interface Alarm {
	id: number;
	device_id: string;
	subsystem: string;
	severity: 'critical' | 'warning' | 'info';
	category: string;
	message: string;
	value: number;
	threshold: number;
	status: 'active' | 'acknowledged' | 'cleared';
	occurred_at: string;
	acked_at?: string;
	acked_by?: string;
	cleared_at?: string;
}

export interface AlarmCounts {
	critical: number;
	warning: number;
	info: number;
}

// ── WebSocket Messages ──

export interface WSMessage {
	type: 'telemetry' | 'alarm' | 'device_status' | 'command_response';
	subsystem?: string;
	device_id?: string;
	data: unknown;
	timestamp: string;
}

// ── Auth ──

export interface User {
	id: string;
	username: string;
	email: string;
	role: 'admin' | 'operator' | 'viewer';
	active: boolean;
}

export interface AuthState {
	user: User | null;
	token: string | null;
	isAuthenticated: boolean;
}

// ── Aggregated Data ──

export interface AggregatedReading {
	timestamp: string;
	avg_value: number;
	min_value: number;
	max_value: number;
	sample_count: number;
}
