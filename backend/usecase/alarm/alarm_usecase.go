package alarm

import (
	"context"
	"fmt"
	"sync"
	"time"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
	"scada-system/domain/valueobject"
)

// AlarmRule defines a condition that triggers an alarm
type AlarmRule struct {
	ID         string
	Subsystem  entity.Subsystem
	Metric     string
	Condition  string // gt, lt, gte, lte
	Threshold  float64
	Severity   entity.AlarmSeverity
	Category   entity.AlarmCategory
	Message    string
	Cooldown   time.Duration
}

type UseCase struct {
	alarmRepo   repository.AlarmRepository
	broadcaster repository.RealtimeBroadcaster
	rules       []AlarmRule
	rulesMu     sync.RWMutex
	lastFired   map[string]time.Time
	lastFiredMu sync.Mutex
}

func NewUseCase(alarmRepo repository.AlarmRepository, broadcaster repository.RealtimeBroadcaster) *UseCase {
	uc := &UseCase{
		alarmRepo: alarmRepo,
		broadcaster: broadcaster,
		lastFired: make(map[string]time.Time),
	}
	uc.loadDefaultRules()
	return uc
}

func (uc *UseCase) GetActive(ctx context.Context, subsystem string) ([]entity.Alarm, error) {
	return uc.alarmRepo.FindActive(ctx, subsystem)
}

func (uc *UseCase) GetHistory(ctx context.Context, subsystem string, tr valueobject.TimeRange, limit int) ([]entity.Alarm, error) {
	return uc.alarmRepo.FindHistory(ctx, subsystem, tr, limit)
}

func (uc *UseCase) Acknowledge(ctx context.Context, alarmID int64, userID string) error {
	if userID == "" {
		return domainerr.NewValidation("user_id", "required for acknowledgement")
	}
	return uc.alarmRepo.Acknowledge(ctx, alarmID, userID)
}

func (uc *UseCase) Clear(ctx context.Context, alarmID int64) error {
	return uc.alarmRepo.Clear(ctx, alarmID)
}

func (uc *UseCase) GetCounts(ctx context.Context) (valueobject.AlarmCounts, error) {
	return uc.alarmRepo.GetCounts(ctx)
}

// EvaluateMetric checks a metric value against all matching alarm rules
func (uc *UseCase) EvaluateMetric(ctx context.Context, deviceID string, subsystem entity.Subsystem, metric string, value float64) {
	uc.rulesMu.RLock()
	defer uc.rulesMu.RUnlock()

	for _, rule := range uc.rules {
		if rule.Subsystem != subsystem || rule.Metric != metric {
			continue
		}
		if uc.checkCondition(rule, value) {
			uc.fireAlarm(ctx, deviceID, rule, value)
		}
	}
}

func (uc *UseCase) checkCondition(rule AlarmRule, value float64) bool {
	switch rule.Condition {
	case "gt":
		return value > rule.Threshold
	case "lt":
		return value < rule.Threshold
	case "gte":
		return value >= rule.Threshold
	case "lte":
		return value <= rule.Threshold
	default:
		return false
	}
}

func (uc *UseCase) fireAlarm(ctx context.Context, deviceID string, rule AlarmRule, value float64) {
	key := rule.ID + ":" + deviceID
	uc.lastFiredMu.Lock()
	defer uc.lastFiredMu.Unlock()

	if lastTime, ok := uc.lastFired[key]; ok && time.Since(lastTime) < rule.Cooldown {
		return
	}
	uc.lastFired[key] = time.Now()

	alarm := &entity.Alarm{
		DeviceID:  deviceID,
		Subsystem: rule.Subsystem,
		Severity:  rule.Severity,
		Category:  rule.Category,
		Message:   fmt.Sprintf("%s (value: %.2f, threshold: %.2f)", rule.Message, value, rule.Threshold),
		Value:     value,
		Threshold: rule.Threshold,
		Status:    entity.AlarmActive,
	}

	if err := uc.alarmRepo.Create(ctx, alarm); err != nil {
		return
	}
	uc.broadcaster.BroadcastAlarm(alarm)
}

func (uc *UseCase) loadDefaultRules() {
	uc.rules = []AlarmRule{
		{ID: "water_tank_high", Subsystem: entity.SubsystemWater, Metric: "tank_level", Condition: "gt", Threshold: 90, Severity: entity.SeverityWarning, Category: entity.CategoryProcess, Message: "Water tank level HIGH", Cooldown: 5 * time.Minute},
		{ID: "water_tank_critical", Subsystem: entity.SubsystemWater, Metric: "tank_level", Condition: "gt", Threshold: 95, Severity: entity.SeverityCritical, Category: entity.CategoryProcess, Message: "Water tank CRITICAL - overflow risk", Cooldown: 2 * time.Minute},
		{ID: "water_tank_low", Subsystem: entity.SubsystemWater, Metric: "tank_level", Condition: "lt", Threshold: 20, Severity: entity.SeverityWarning, Category: entity.CategoryProcess, Message: "Water tank level LOW", Cooldown: 5 * time.Minute},
		{ID: "water_ph_high", Subsystem: entity.SubsystemWater, Metric: "ph_level", Condition: "gt", Threshold: 8.5, Severity: entity.SeverityWarning, Category: entity.CategoryProcess, Message: "pH level above range", Cooldown: 10 * time.Minute},
		{ID: "water_ph_low", Subsystem: entity.SubsystemWater, Metric: "ph_level", Condition: "lt", Threshold: 6.5, Severity: entity.SeverityWarning, Category: entity.CategoryProcess, Message: "pH level below range", Cooldown: 10 * time.Minute},
		{ID: "water_pressure", Subsystem: entity.SubsystemWater, Metric: "pressure", Condition: "gt", Threshold: 6.0, Severity: entity.SeverityCritical, Category: entity.CategoryEquipment, Message: "Pressure exceeds safe limit", Cooldown: 2 * time.Minute},
		{ID: "solar_temp_high", Subsystem: entity.SubsystemSolar, Metric: "panel_temperature", Condition: "gt", Threshold: 85, Severity: entity.SeverityCritical, Category: entity.CategoryEquipment, Message: "Panel temperature exceeds limit", Cooldown: 5 * time.Minute},
		{ID: "solar_freq_high", Subsystem: entity.SubsystemSolar, Metric: "grid_frequency", Condition: "gt", Threshold: 50.5, Severity: entity.SeverityCritical, Category: entity.CategoryProcess, Message: "Grid frequency HIGH", Cooldown: 1 * time.Minute},
		{ID: "solar_freq_low", Subsystem: entity.SubsystemSolar, Metric: "grid_frequency", Condition: "lt", Threshold: 49.5, Severity: entity.SeverityCritical, Category: entity.CategoryProcess, Message: "Grid frequency LOW", Cooldown: 1 * time.Minute},
	}
}
