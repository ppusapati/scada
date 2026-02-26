package entity

import "time"

// Device represents a physical IoT device in the SCADA system
type Device struct {
	ID         string
	Name       string
	DeviceType string
	Location   string
	Subsystem  Subsystem
	Status     DeviceStatus
	LastSeen   time.Time
	Metadata   map[string]interface{}
	CreatedAt  time.Time
	UpdatedAt  time.Time
}

// SensorReading is an immutable time-series data point
type SensorReading struct {
	ID        int64
	DeviceID  string
	Timestamp time.Time
	Metric    string
	Value     float64
	Unit      string
	Quality   DataQuality
}

// WaterSystem holds water treatment plant state
type WaterSystem struct {
	ID            string
	Name          string
	TankLevel     float64
	FlowRateIn    float64
	FlowRateOut   float64
	Pressure      float64
	PHLevel       float64
	Turbidity     float64
	ChlorineLevel float64
	PumpStatus    PumpStatus
	ValvePositions map[string]float64
	OperatingMode OperatingMode
	UpdatedAt     time.Time
}

// SolarSystem holds solar plant state
type SolarSystem struct {
	ID               string
	Name             string
	TotalCapacityKW  float64
	CurrentOutputKW  float64
	Irradiance       float64
	PanelTemperature float64
	InverterStatus   string
	GridVoltage      float64
	GridFrequency    float64
	DailyEnergyKWH  float64
	TotalEnergyMWH  float64
	Efficiency       float64
	OperatingMode    OperatingMode
	UpdatedAt        time.Time
}

// Alarm represents a system alarm event
type Alarm struct {
	ID         int64
	DeviceID   string
	Subsystem  Subsystem
	Severity   AlarmSeverity
	Category   AlarmCategory
	Message    string
	Value      float64
	Threshold  float64
	Status     AlarmStatus
	OccurredAt time.Time
	AckedAt    *time.Time
	AckedBy    string
	ClearedAt  *time.Time
}

// User represents a SCADA operator
type User struct {
	ID           string
	Username     string
	Email        string
	PasswordHash string
	Role         UserRole
	Active       bool
	CreatedAt    time.Time
	LastLogin    *time.Time
}

// Command represents a control command to a device
type Command struct {
	ID          int64
	DeviceID    string
	UserID      string
	Action      string
	Parameters  map[string]string
	Status      CommandStatus
	IssuedAt    time.Time
	CompletedAt *time.Time
}

// SetpointConfig for process control limits
type SetpointConfig struct {
	ID        string
	Subsystem Subsystem
	Parameter string
	Value     float64
	MinValue  float64
	MaxValue  float64
	Unit      string
	UpdatedBy string
}
