package entity

// Subsystem identifies which part of the SCADA system
type Subsystem string

const (
	SubsystemWater Subsystem = "water"
	SubsystemSolar Subsystem = "solar"
)

func (s Subsystem) IsValid() bool {
	return s == SubsystemWater || s == SubsystemSolar
}

func (s Subsystem) String() string { return string(s) }

// DeviceStatus represents operational state
type DeviceStatus string

const (
	DeviceOnline      DeviceStatus = "online"
	DeviceOffline     DeviceStatus = "offline"
	DeviceFault       DeviceStatus = "fault"
	DeviceMaintenance DeviceStatus = "maintenance"
)

// DataQuality indicates sensor reading reliability
type DataQuality int

const (
	QualityGood      DataQuality = 0
	QualityUncertain DataQuality = 1
	QualityBad       DataQuality = 2
)

// PumpStatus for water subsystem
type PumpStatus string

const (
	PumpRunning PumpStatus = "running"
	PumpStopped PumpStatus = "stopped"
	PumpFault   PumpStatus = "fault"
)

// OperatingMode for subsystem control
type OperatingMode string

const (
	ModeAuto        OperatingMode = "auto"
	ModeManual      OperatingMode = "manual"
	ModeMaintenance OperatingMode = "maintenance"
)

func (m OperatingMode) IsValid() bool {
	return m == ModeAuto || m == ModeManual || m == ModeMaintenance
}

// AlarmSeverity levels
type AlarmSeverity string

const (
	SeverityCritical AlarmSeverity = "critical"
	SeverityWarning  AlarmSeverity = "warning"
	SeverityInfo     AlarmSeverity = "info"
)

// AlarmCategory classification
type AlarmCategory string

const (
	CategoryProcess       AlarmCategory = "process"
	CategoryEquipment     AlarmCategory = "equipment"
	CategoryCommunication AlarmCategory = "communication"
	CategorySecurity      AlarmCategory = "security"
)

// AlarmStatus lifecycle
type AlarmStatus string

const (
	AlarmActive       AlarmStatus = "active"
	AlarmAcknowledged AlarmStatus = "acknowledged"
	AlarmCleared      AlarmStatus = "cleared"
)

// CommandStatus lifecycle
type CommandStatus string

const (
	CommandPending      CommandStatus = "pending"
	CommandSent         CommandStatus = "sent"
	CommandAcknowledged CommandStatus = "acknowledged"
	CommandCompleted    CommandStatus = "completed"
	CommandFailed       CommandStatus = "failed"
)

// UserRole for RBAC
type UserRole string

const (
	RoleAdmin    UserRole = "admin"
	RoleOperator UserRole = "operator"
	RoleViewer   UserRole = "viewer"
)

func (r UserRole) CanControl() bool {
	return r == RoleAdmin || r == RoleOperator
}

func (r UserRole) IsAdmin() bool {
	return r == RoleAdmin
}
