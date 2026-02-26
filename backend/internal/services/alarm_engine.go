package services

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
	"scada-system/internal/websocket"
)

// AlarmRule defines a condition that triggers an alarm
type AlarmRule struct {
	ID         string
	Subsystem  string
	Metric     string
	Condition  string  // gt, lt, gte, lte, eq, ne
	Threshold  float64
	Severity   string  // critical, warning, info
	Category   string  // process, equipment
	Message    string
	Hysteresis float64 // Prevents alarm flapping
	Cooldown   time.Duration
}

// AlarmEngine evaluates sensor readings against rules and generates alarms
type AlarmEngine struct {
	pool       *pgxpool.Pool
	alarmSvc   *AlarmService
	wsHub      *websocket.Hub
	rules      []AlarmRule
	rulesMu    sync.RWMutex
	lastFired  map[string]time.Time // rule_id+device_id -> last alarm time
	lastFiredMu sync.Mutex
}

func NewAlarmEngine(pool *pgxpool.Pool, alarmSvc *AlarmService, wsHub *websocket.Hub) *AlarmEngine {
	engine := &AlarmEngine{
		pool:      pool,
		alarmSvc:  alarmSvc,
		wsHub:     wsHub,
		lastFired: make(map[string]time.Time),
	}
	engine.loadDefaultRules()
	return engine
}

func (e *AlarmEngine) loadDefaultRules() {
	e.rules = []AlarmRule{
		// ── Water Alarms ──
		{ID: "water_tank_high", Subsystem: "water", Metric: "tank_level", Condition: "gt", Threshold: 90, Severity: "warning", Category: "process", Message: "Water tank level HIGH", Hysteresis: 2, Cooldown: 5 * time.Minute},
		{ID: "water_tank_critical", Subsystem: "water", Metric: "tank_level", Condition: "gt", Threshold: 95, Severity: "critical", Category: "process", Message: "Water tank level CRITICAL - overflow risk", Hysteresis: 1, Cooldown: 2 * time.Minute},
		{ID: "water_tank_low", Subsystem: "water", Metric: "tank_level", Condition: "lt", Threshold: 20, Severity: "warning", Category: "process", Message: "Water tank level LOW", Hysteresis: 2, Cooldown: 5 * time.Minute},
		{ID: "water_tank_empty", Subsystem: "water", Metric: "tank_level", Condition: "lt", Threshold: 10, Severity: "critical", Category: "process", Message: "Water tank level CRITICAL - near empty", Hysteresis: 1, Cooldown: 2 * time.Minute},
		{ID: "water_ph_high", Subsystem: "water", Metric: "ph_level", Condition: "gt", Threshold: 8.5, Severity: "warning", Category: "process", Message: "pH level above normal range", Hysteresis: 0.1, Cooldown: 10 * time.Minute},
		{ID: "water_ph_low", Subsystem: "water", Metric: "ph_level", Condition: "lt", Threshold: 6.5, Severity: "warning", Category: "process", Message: "pH level below normal range", Hysteresis: 0.1, Cooldown: 10 * time.Minute},
		{ID: "water_pressure_high", Subsystem: "water", Metric: "pressure", Condition: "gt", Threshold: 6.0, Severity: "critical", Category: "equipment", Message: "System pressure exceeds safe limit", Hysteresis: 0.2, Cooldown: 2 * time.Minute},
		{ID: "water_turbidity_high", Subsystem: "water", Metric: "turbidity", Condition: "gt", Threshold: 4.0, Severity: "warning", Category: "process", Message: "Turbidity above acceptable level", Hysteresis: 0.3, Cooldown: 10 * time.Minute},
		{ID: "water_chlorine_low", Subsystem: "water", Metric: "chlorine_level", Condition: "lt", Threshold: 0.2, Severity: "warning", Category: "process", Message: "Chlorine level too low - disinfection risk", Hysteresis: 0.05, Cooldown: 15 * time.Minute},

		// ── Solar Alarms ──
		{ID: "solar_panel_temp_high", Subsystem: "solar", Metric: "panel_temperature", Condition: "gt", Threshold: 85, Severity: "critical", Category: "equipment", Message: "Panel temperature exceeds safe limit", Hysteresis: 2, Cooldown: 5 * time.Minute},
		{ID: "solar_grid_voltage_high", Subsystem: "solar", Metric: "grid_voltage", Condition: "gt", Threshold: 253, Severity: "warning", Category: "process", Message: "Grid voltage above normal range", Hysteresis: 2, Cooldown: 5 * time.Minute},
		{ID: "solar_grid_voltage_low", Subsystem: "solar", Metric: "grid_voltage", Condition: "lt", Threshold: 207, Severity: "warning", Category: "process", Message: "Grid voltage below normal range", Hysteresis: 2, Cooldown: 5 * time.Minute},
		{ID: "solar_grid_freq_high", Subsystem: "solar", Metric: "grid_frequency", Condition: "gt", Threshold: 50.5, Severity: "critical", Category: "process", Message: "Grid frequency HIGH - disconnect required", Hysteresis: 0.1, Cooldown: 1 * time.Minute},
		{ID: "solar_grid_freq_low", Subsystem: "solar", Metric: "grid_frequency", Condition: "lt", Threshold: 49.5, Severity: "critical", Category: "process", Message: "Grid frequency LOW - disconnect required", Hysteresis: 0.1, Cooldown: 1 * time.Minute},
	}
}

// Evaluate checks a single metric value against all matching rules
func (e *AlarmEngine) Evaluate(ctx context.Context, deviceID, subsystem, metric string, value float64) {
	e.rulesMu.RLock()
	defer e.rulesMu.RUnlock()

	for _, rule := range e.rules {
		if rule.Subsystem != subsystem || rule.Metric != metric {
			continue
		}

		triggered := e.checkCondition(rule, value)
		if triggered {
			e.fireAlarm(ctx, deviceID, rule, value)
		}
	}
}

func (e *AlarmEngine) checkCondition(rule AlarmRule, value float64) bool {
	switch rule.Condition {
	case "gt":
		return value > rule.Threshold
	case "lt":
		return value < rule.Threshold
	case "gte":
		return value >= rule.Threshold
	case "lte":
		return value <= rule.Threshold
	case "eq":
		return value == rule.Threshold
	case "ne":
		return value != rule.Threshold
	default:
		return false
	}
}

func (e *AlarmEngine) fireAlarm(ctx context.Context, deviceID string, rule AlarmRule, value float64) {
	key := rule.ID + ":" + deviceID
	e.lastFiredMu.Lock()
	defer e.lastFiredMu.Unlock()

	// Check cooldown
	if lastTime, ok := e.lastFired[key]; ok {
		if time.Since(lastTime) < rule.Cooldown {
			return // Still in cooldown
		}
	}

	e.lastFired[key] = time.Now()

	alarm := &models.Alarm{
		DeviceID:  deviceID,
		Subsystem: rule.Subsystem,
		Severity:  rule.Severity,
		Category:  rule.Category,
		Message:   fmt.Sprintf("%s (value: %.2f, threshold: %.2f)", rule.Message, value, rule.Threshold),
		Value:     value,
		Threshold: rule.Threshold,
	}

	if err := e.alarmSvc.Create(ctx, alarm); err != nil {
		fmt.Printf("[AlarmEngine] Failed to create alarm: %v\n", err)
		return
	}

	// Broadcast to WebSocket clients
	e.wsHub.Broadcast(websocket.Message{
		Type:      "alarm",
		Subsystem: rule.Subsystem,
		DeviceID:  deviceID,
		Data:      alarm,
		Timestamp: time.Now(),
	})

	fmt.Printf("[AlarmEngine] ALARM: %s [%s] %s\n", rule.Severity, rule.Subsystem, alarm.Message)
}

// AddRule dynamically adds a new alarm rule
func (e *AlarmEngine) AddRule(rule AlarmRule) {
	e.rulesMu.Lock()
	defer e.rulesMu.Unlock()
	e.rules = append(e.rules, rule)
}

// GetRules returns a copy of all current rules
func (e *AlarmEngine) GetRules() []AlarmRule {
	e.rulesMu.RLock()
	defer e.rulesMu.RUnlock()
	rules := make([]AlarmRule, len(e.rules))
	copy(rules, e.rules)
	return rules
}
