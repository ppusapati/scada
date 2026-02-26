-- SCADA System Database Schema
-- Requires PostgreSQL with TimescaleDB extension

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- ============================================================
-- Users & Authentication
-- ============================================================
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(64) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'viewer' CHECK (role IN ('admin', 'operator', 'viewer')),
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login TIMESTAMPTZ
);

-- ============================================================
-- Devices
-- ============================================================
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(128) NOT NULL,
    device_type VARCHAR(64) NOT NULL,
    location VARCHAR(255),
    subsystem VARCHAR(20) NOT NULL CHECK (subsystem IN ('water', 'solar')),
    status VARCHAR(20) NOT NULL DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'fault', 'maintenance')),
    last_seen TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devices_subsystem ON devices(subsystem);
CREATE INDEX idx_devices_status ON devices(status);

-- ============================================================
-- Sensor Readings (TimescaleDB hypertable)
-- ============================================================
CREATE TABLE sensor_readings (
    id BIGSERIAL,
    device_id UUID NOT NULL REFERENCES devices(id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metric VARCHAR(64) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit VARCHAR(20) NOT NULL,
    quality SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id, timestamp)
);

SELECT create_hypertable('sensor_readings', 'timestamp');

CREATE INDEX idx_readings_device_metric ON sensor_readings(device_id, metric, timestamp DESC);
CREATE INDEX idx_readings_metric ON sensor_readings(metric, timestamp DESC);

-- ============================================================
-- Water Subsystem State
-- ============================================================
CREATE TABLE water_systems (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(128) NOT NULL,
    tank_level DOUBLE PRECISION DEFAULT 0,
    flow_rate_in DOUBLE PRECISION DEFAULT 0,
    flow_rate_out DOUBLE PRECISION DEFAULT 0,
    pressure DOUBLE PRECISION DEFAULT 0,
    ph_level DOUBLE PRECISION DEFAULT 7.0,
    turbidity DOUBLE PRECISION DEFAULT 0,
    chlorine_level DOUBLE PRECISION DEFAULT 0,
    pump_status VARCHAR(20) DEFAULT 'stopped',
    valve_positions JSONB DEFAULT '{}',
    operating_mode VARCHAR(20) DEFAULT 'auto' CHECK (operating_mode IN ('auto', 'manual', 'maintenance')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Solar Subsystem State
-- ============================================================
CREATE TABLE solar_systems (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(128) NOT NULL,
    total_capacity_kw DOUBLE PRECISION DEFAULT 0,
    current_output_kw DOUBLE PRECISION DEFAULT 0,
    irradiance DOUBLE PRECISION DEFAULT 0,
    panel_temperature DOUBLE PRECISION DEFAULT 0,
    inverter_status VARCHAR(20) DEFAULT 'offline',
    grid_voltage DOUBLE PRECISION DEFAULT 0,
    grid_frequency DOUBLE PRECISION DEFAULT 0,
    daily_energy_kwh DOUBLE PRECISION DEFAULT 0,
    total_energy_mwh DOUBLE PRECISION DEFAULT 0,
    efficiency DOUBLE PRECISION DEFAULT 0,
    operating_mode VARCHAR(20) DEFAULT 'auto',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Alarms
-- ============================================================
CREATE TABLE alarms (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id),
    subsystem VARCHAR(20) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('critical', 'warning', 'info')),
    category VARCHAR(30) NOT NULL CHECK (category IN ('process', 'equipment', 'communication', 'security')),
    message TEXT NOT NULL,
    value DOUBLE PRECISION,
    threshold DOUBLE PRECISION,
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'acknowledged', 'cleared')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acked_at TIMESTAMPTZ,
    acked_by UUID REFERENCES users(id),
    cleared_at TIMESTAMPTZ
);

CREATE INDEX idx_alarms_status ON alarms(status);
CREATE INDEX idx_alarms_subsystem ON alarms(subsystem, occurred_at DESC);
CREATE INDEX idx_alarms_severity ON alarms(severity, status);

-- ============================================================
-- Control Commands Audit Log
-- ============================================================
CREATE TABLE commands (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id),
    user_id UUID NOT NULL REFERENCES users(id),
    action VARCHAR(64) NOT NULL,
    parameters JSONB DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'acknowledged', 'completed', 'failed')),
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_commands_device ON commands(device_id, issued_at DESC);

-- ============================================================
-- Setpoint Configuration
-- ============================================================
CREATE TABLE setpoint_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subsystem VARCHAR(20) NOT NULL,
    parameter VARCHAR(64) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    min_value DOUBLE PRECISION NOT NULL,
    max_value DOUBLE PRECISION NOT NULL,
    unit VARCHAR(20) NOT NULL,
    updated_by UUID REFERENCES users(id),
    UNIQUE(subsystem, parameter)
);

-- ============================================================
-- Seed Data
-- ============================================================
INSERT INTO users (username, email, password_hash, role) VALUES
    ('admin', 'admin@scada.local', '$2a$10$placeholder_hash_change_me', 'admin');

-- Default Water System
INSERT INTO water_systems (name, tank_level, ph_level, operating_mode) VALUES
    ('Main Water Treatment Plant', 65.0, 7.2, 'auto');

-- Default Solar System
INSERT INTO solar_systems (name, total_capacity_kw, operating_mode) VALUES
    ('Solar Array Alpha', 500.0, 'auto');

-- Default Setpoints for Water
INSERT INTO setpoint_configs (subsystem, parameter, value, min_value, max_value, unit) VALUES
    ('water', 'tank_level_high', 90.0, 50.0, 100.0, '%'),
    ('water', 'tank_level_low', 20.0, 0.0, 50.0, '%'),
    ('water', 'ph_high', 8.5, 7.0, 14.0, 'pH'),
    ('water', 'ph_low', 6.5, 0.0, 7.0, 'pH'),
    ('water', 'pressure_max', 6.0, 0.0, 10.0, 'bar'),
    ('water', 'turbidity_max', 4.0, 0.0, 10.0, 'NTU'),
    ('water', 'chlorine_min', 0.2, 0.0, 2.0, 'mg/L'),
    ('water', 'chlorine_max', 1.5, 0.0, 5.0, 'mg/L');

-- Default Setpoints for Solar
INSERT INTO setpoint_configs (subsystem, parameter, value, min_value, max_value, unit) VALUES
    ('solar', 'panel_temp_max', 85.0, 0.0, 100.0, '°C'),
    ('solar', 'grid_voltage_high', 253.0, 200.0, 260.0, 'V'),
    ('solar', 'grid_voltage_low', 207.0, 190.0, 240.0, 'V'),
    ('solar', 'grid_freq_high', 50.5, 49.0, 51.0, 'Hz'),
    ('solar', 'grid_freq_low', 49.5, 49.0, 51.0, 'Hz'),
    ('solar', 'efficiency_min', 15.0, 0.0, 100.0, '%');
