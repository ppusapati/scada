package models

import "time"

// Device represents a physical IoT device in the SCADA system
type Device struct {
	ID          string    `json:"id" db:"id"`
	Name        string    `json:"name" db:"name"`
	DeviceType  string    `json:"device_type" db:"device_type"` // water_sensor, solar_panel, flow_meter, pump, valve
	Location    string    `json:"location" db:"location"`
	Subsystem   string    `json:"subsystem" db:"subsystem"` // water, solar
	Status      string    `json:"status" db:"status"`       // online, offline, fault, maintenance
	LastSeen    time.Time `json:"last_seen" db:"last_seen"`
	Metadata    string    `json:"metadata" db:"metadata"` // JSON blob for device-specific config
	CreatedAt   time.Time `json:"created_at" db:"created_at"`
	UpdatedAt   time.Time `json:"updated_at" db:"updated_at"`
}

// SensorReading is a time-series data point from a device
type SensorReading struct {
	ID        int64     `json:"id" db:"id"`
	DeviceID  string    `json:"device_id" db:"device_id"`
	Timestamp time.Time `json:"timestamp" db:"timestamp"`
	Metric    string    `json:"metric" db:"metric"` // temperature, pressure, flow_rate, voltage, current, power, irradiance
	Value     float64   `json:"value" db:"value"`
	Unit      string    `json:"unit" db:"unit"`
	Quality   int       `json:"quality" db:"quality"` // 0=good, 1=uncertain, 2=bad
}

// WaterSystem holds water treatment plant operational state
type WaterSystem struct {
	ID                string    `json:"id" db:"id"`
	Name              string    `json:"name" db:"name"`
	TankLevel         float64   `json:"tank_level" db:"tank_level"`                   // percentage
	FlowRateIn        float64   `json:"flow_rate_in" db:"flow_rate_in"`               // L/min
	FlowRateOut       float64   `json:"flow_rate_out" db:"flow_rate_out"`             // L/min
	Pressure          float64   `json:"pressure" db:"pressure"`                       // bar
	PHLevel           float64   `json:"ph_level" db:"ph_level"`
	Turbidity         float64   `json:"turbidity" db:"turbidity"`                     // NTU
	ChlorineLevel     float64   `json:"chlorine_level" db:"chlorine_level"`           // mg/L
	PumpStatus        string    `json:"pump_status" db:"pump_status"`                 // running, stopped, fault
	ValvePositions    string    `json:"valve_positions" db:"valve_positions"`          // JSON
	OperatingMode     string    `json:"operating_mode" db:"operating_mode"`           // auto, manual, maintenance
	UpdatedAt         time.Time `json:"updated_at" db:"updated_at"`
}

// SolarSystem holds solar plant operational state
type SolarSystem struct {
	ID                string    `json:"id" db:"id"`
	Name              string    `json:"name" db:"name"`
	TotalCapacityKW   float64   `json:"total_capacity_kw" db:"total_capacity_kw"`
	CurrentOutputKW   float64   `json:"current_output_kw" db:"current_output_kw"`
	Irradiance        float64   `json:"irradiance" db:"irradiance"`                 // W/m²
	PanelTemperature  float64   `json:"panel_temperature" db:"panel_temperature"`   // °C
	InverterStatus    string    `json:"inverter_status" db:"inverter_status"`       // online, offline, fault
	GridVoltage       float64   `json:"grid_voltage" db:"grid_voltage"`             // V
	GridFrequency     float64   `json:"grid_frequency" db:"grid_frequency"`         // Hz
	DailyEnergyKWH    float64   `json:"daily_energy_kwh" db:"daily_energy_kwh"`
	TotalEnergyMWH    float64   `json:"total_energy_mwh" db:"total_energy_mwh"`
	Efficiency        float64   `json:"efficiency" db:"efficiency"`                 // percentage
	OperatingMode     string    `json:"operating_mode" db:"operating_mode"`
	UpdatedAt         time.Time `json:"updated_at" db:"updated_at"`
}

// Alarm represents a system alarm/event
type Alarm struct {
	ID          int64     `json:"id" db:"id"`
	DeviceID    string    `json:"device_id" db:"device_id"`
	Subsystem   string    `json:"subsystem" db:"subsystem"`
	Severity    string    `json:"severity" db:"severity"` // critical, warning, info
	Category    string    `json:"category" db:"category"` // process, equipment, communication, security
	Message     string    `json:"message" db:"message"`
	Value       float64   `json:"value" db:"value"`
	Threshold   float64   `json:"threshold" db:"threshold"`
	Status      string    `json:"status" db:"status"` // active, acknowledged, cleared
	OccurredAt  time.Time `json:"occurred_at" db:"occurred_at"`
	AckedAt     *time.Time `json:"acked_at,omitempty" db:"acked_at"`
	AckedBy     string    `json:"acked_by,omitempty" db:"acked_by"`
	ClearedAt   *time.Time `json:"cleared_at,omitempty" db:"cleared_at"`
}

// User represents a SCADA system operator
type User struct {
	ID           string    `json:"id" db:"id"`
	Username     string    `json:"username" db:"username"`
	Email        string    `json:"email" db:"email"`
	PasswordHash string    `json:"-" db:"password_hash"`
	Role         string    `json:"role" db:"role"` // admin, operator, viewer
	Active       bool      `json:"active" db:"active"`
	CreatedAt    time.Time `json:"created_at" db:"created_at"`
	LastLogin    *time.Time `json:"last_login,omitempty" db:"last_login"`
}

// Command represents a control command sent to a device
type Command struct {
	ID         int64     `json:"id" db:"id"`
	DeviceID   string    `json:"device_id" db:"device_id"`
	UserID     string    `json:"user_id" db:"user_id"`
	Action     string    `json:"action" db:"action"`     // start_pump, stop_pump, open_valve, close_valve, set_setpoint
	Parameters string    `json:"parameters" db:"parameters"` // JSON
	Status     string    `json:"status" db:"status"`     // pending, sent, acknowledged, completed, failed
	IssuedAt   time.Time `json:"issued_at" db:"issued_at"`
	CompletedAt *time.Time `json:"completed_at,omitempty" db:"completed_at"`
}

// SetpointConfig represents configurable setpoints for process control
type SetpointConfig struct {
	ID         string  `json:"id" db:"id"`
	Subsystem  string  `json:"subsystem" db:"subsystem"`
	Parameter  string  `json:"parameter" db:"parameter"`
	Value      float64 `json:"value" db:"value"`
	MinValue   float64 `json:"min_value" db:"min_value"`
	MaxValue   float64 `json:"max_value" db:"max_value"`
	Unit       string  `json:"unit" db:"unit"`
	UpdatedBy  string  `json:"updated_by" db:"updated_by"`
}
