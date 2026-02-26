package repository

import (
	"context"

	"scada-system/domain/entity"
	"scada-system/domain/valueobject"
)

// DeviceRepository — port for device persistence
type DeviceRepository interface {
	FindAll(ctx context.Context, subsystem string) ([]entity.Device, error)
	FindByID(ctx context.Context, id string) (*entity.Device, error)
	Create(ctx context.Context, device *entity.Device) error
	UpdateStatus(ctx context.Context, id string, status entity.DeviceStatus) error
	Delete(ctx context.Context, id string) error
}

// ReadingRepository — port for time-series sensor data
type ReadingRepository interface {
	Insert(ctx context.Context, reading *entity.SensorReading) error
	InsertBatch(ctx context.Context, readings []entity.SensorReading) error
	FindLatest(ctx context.Context, deviceID, metric string) (*entity.SensorReading, error)
	FindHistory(ctx context.Context, deviceID, metric string, timeRange valueobject.TimeRange, limit int) ([]entity.SensorReading, error)
	FindAggregated(ctx context.Context, deviceID, metric string, interval valueobject.AggregationInterval, timeRange valueobject.TimeRange) ([]valueobject.AggregatedReading, error)
}

// WaterRepository — port for water subsystem state
type WaterRepository interface {
	FindAll(ctx context.Context) ([]entity.WaterSystem, error)
	FindByID(ctx context.Context, id string) (*entity.WaterSystem, error)
	UpdateFromTelemetry(ctx context.Context, id string, updates map[string]interface{}) error
	SetOperatingMode(ctx context.Context, id string, mode entity.OperatingMode) error
	SetPumpStatus(ctx context.Context, id string, status entity.PumpStatus) error
}

// SolarRepository — port for solar subsystem state
type SolarRepository interface {
	FindAll(ctx context.Context) ([]entity.SolarSystem, error)
	FindByID(ctx context.Context, id string) (*entity.SolarSystem, error)
	UpdateFromTelemetry(ctx context.Context, id string, updates map[string]interface{}) error
	SetOperatingMode(ctx context.Context, id string, mode entity.OperatingMode) error
}

// AlarmRepository — port for alarm persistence
type AlarmRepository interface {
	Create(ctx context.Context, alarm *entity.Alarm) error
	FindActive(ctx context.Context, subsystem string) ([]entity.Alarm, error)
	FindHistory(ctx context.Context, subsystem string, timeRange valueobject.TimeRange, limit int) ([]entity.Alarm, error)
	Acknowledge(ctx context.Context, alarmID int64, userID string) error
	Clear(ctx context.Context, alarmID int64) error
	GetCounts(ctx context.Context) (valueobject.AlarmCounts, error)
}

// UserRepository — port for user/auth persistence
type UserRepository interface {
	FindByUsername(ctx context.Context, username string) (*entity.User, error)
	FindByID(ctx context.Context, id string) (*entity.User, error)
	Create(ctx context.Context, user *entity.User) error
	UpdateLastLogin(ctx context.Context, id string) error
	FindAll(ctx context.Context) ([]entity.User, error)
}

// CommandRepository — port for command audit log
type CommandRepository interface {
	Create(ctx context.Context, cmd *entity.Command) error
	UpdateStatus(ctx context.Context, id int64, status entity.CommandStatus) error
	FindByDevice(ctx context.Context, deviceID string, limit int) ([]entity.Command, error)
}

// MQTTPublisher — port for outbound MQTT messages
type MQTTPublisher interface {
	PublishCommand(ctx context.Context, subsystem, deviceID string, cmd valueobject.CommandRequest) error
	PublishStatus(ctx context.Context, subsystem, deviceID, status string) error
}

// RealtimeBroadcaster — port for WebSocket push
type RealtimeBroadcaster interface {
	BroadcastTelemetry(subsystem, deviceID string, data interface{})
	BroadcastAlarm(alarm *entity.Alarm)
	BroadcastDeviceStatus(subsystem, deviceID, status string)
}

// ReportRepository — port for reporting queries
type ReportRepository interface {
	GetMetricSummary(ctx context.Context, subsystem, metric string, timeRange valueobject.TimeRange) (*valueobject.MetricSummary, error)
	GetAlarmSummary(ctx context.Context, timeRange valueobject.TimeRange) (map[string]int, map[string]int, float64, error)
	GetUptime(ctx context.Context, timeRange valueobject.TimeRange) (float64, error)
}
