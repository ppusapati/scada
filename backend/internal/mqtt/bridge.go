package mqtt

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"scada-system/internal/db/models"
	"scada-system/internal/services"
	"scada-system/internal/websocket"
)

// Bridge connects MQTT messages to the database and WebSocket hub
type Bridge struct {
	mqtt       *Client
	readings   *services.ReadingService
	devices    *services.DeviceService
	water      *services.WaterService
	solar      *services.SolarService
	alarms     *services.AlarmService
	wsHub      *websocket.Hub
}

func NewBridge(
	mqttClient *Client,
	readings *services.ReadingService,
	devices *services.DeviceService,
	water *services.WaterService,
	solar *services.SolarService,
	alarms *services.AlarmService,
	wsHub *websocket.Hub,
) *Bridge {
	return &Bridge{
		mqtt:     mqttClient,
		readings: readings,
		devices:  devices,
		water:    water,
		solar:    solar,
		alarms:   alarms,
		wsHub:    wsHub,
	}
}

func (b *Bridge) Start(ctx context.Context) error {
	// Subscribe to all telemetry topics
	if err := b.mqtt.Subscribe("scada/+/+/telemetry", b.handleTelemetry); err != nil {
		return fmt.Errorf("subscribe telemetry: %w", err)
	}

	// Subscribe to device status updates
	if err := b.mqtt.Subscribe("scada/+/+/status", b.handleStatus); err != nil {
		return fmt.Errorf("subscribe status: %w", err)
	}

	// Subscribe to command responses
	if err := b.mqtt.Subscribe("scada/+/+/response", b.handleCommandResponse); err != nil {
		return fmt.Errorf("subscribe response: %w", err)
	}

	// Start heartbeat publisher
	go b.publishHeartbeat(ctx)

	fmt.Println("[Bridge] MQTT-to-DB bridge started")
	return nil
}

func (b *Bridge) handleTelemetry(topic string, payload []byte) {
	parts := strings.Split(topic, "/")
	if len(parts) < 4 {
		return
	}
	subsystem := parts[1] // water or solar
	deviceID := parts[2]

	var data TelemetryData
	if err := json.Unmarshal(payload, &data); err != nil {
		fmt.Printf("[Bridge] Failed to unmarshal telemetry: %v\n", err)
		return
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	// Store each metric as a sensor reading
	for metric, value := range data.Metrics {
		reading := &models.SensorReading{
			DeviceID:  deviceID,
			Timestamp: data.Timestamp,
			Metric:    metric,
			Value:     value,
			Unit:      getUnitForMetric(metric),
			Quality:   data.Quality,
		}
		if err := b.readings.Insert(ctx, reading); err != nil {
			fmt.Printf("[Bridge] Failed to insert reading: %v\n", err)
		}
	}

	// Update subsystem state
	switch subsystem {
	case "water":
		if err := b.water.UpdateFromMQTT(ctx, deviceID, data.Metrics); err != nil {
			fmt.Printf("[Bridge] Failed to update water state: %v\n", err)
		}
	case "solar":
		floatMap := make(map[string]interface{})
		for k, v := range data.Metrics {
			floatMap[k] = v
		}
		if err := b.solar.UpdateFromMQTT(ctx, deviceID, floatMap); err != nil {
			fmt.Printf("[Bridge] Failed to update solar state: %v\n", err)
		}
	}

	// Update device last seen
	b.devices.UpdateStatus(ctx, deviceID, "online")

	// Forward to WebSocket clients
	b.wsHub.Broadcast(websocket.Message{
		Type:      "telemetry",
		Subsystem: subsystem,
		DeviceID:  deviceID,
		Data:      data,
		Timestamp: data.Timestamp,
	})
}

func (b *Bridge) handleStatus(topic string, payload []byte) {
	parts := strings.Split(topic, "/")
	if len(parts) < 4 {
		return
	}

	var data StatusData
	if err := json.Unmarshal(payload, &data); err != nil {
		return
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	b.devices.UpdateStatus(ctx, data.DeviceID, data.Status)

	// If device went offline or faulted, raise an alarm
	if data.Status == "offline" || data.Status == "fault" {
		subsystem := parts[1]
		severity := "warning"
		if data.Status == "fault" {
			severity = "critical"
		}
		alarm := &models.Alarm{
			DeviceID:  data.DeviceID,
			Subsystem: subsystem,
			Severity:  severity,
			Category:  "equipment",
			Message:   fmt.Sprintf("Device %s reported status: %s", data.DeviceID, data.Status),
		}
		b.alarms.Create(ctx, alarm)

		b.wsHub.Broadcast(websocket.Message{
			Type:      "alarm",
			Subsystem: subsystem,
			DeviceID:  data.DeviceID,
			Data:      alarm,
			Timestamp: time.Now(),
		})
	}

	b.wsHub.Broadcast(websocket.Message{
		Type:      "device_status",
		Subsystem: parts[1],
		DeviceID:  data.DeviceID,
		Data:      data,
		Timestamp: time.Now(),
	})
}

func (b *Bridge) handleCommandResponse(topic string, payload []byte) {
	b.wsHub.Broadcast(websocket.Message{
		Type:      "command_response",
		Data:      json.RawMessage(payload),
		Timestamp: time.Now(),
	})
}

func (b *Bridge) publishHeartbeat(ctx context.Context) {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			b.mqtt.Publish("scada/system/heartbeat", map[string]interface{}{
				"timestamp": time.Now(),
				"status":    "online",
			})
		}
	}
}

func getUnitForMetric(metric string) string {
	units := map[string]string{
		"temperature":    "°C",
		"pressure":       "bar",
		"flow_rate":      "L/min",
		"flow_rate_in":   "L/min",
		"flow_rate_out":  "L/min",
		"tank_level":     "%",
		"ph_level":       "pH",
		"turbidity":      "NTU",
		"chlorine_level": "mg/L",
		"voltage":        "V",
		"current":        "A",
		"power":          "kW",
		"irradiance":     "W/m²",
		"energy":         "kWh",
		"frequency":      "Hz",
		"efficiency":     "%",
	}
	if u, ok := units[metric]; ok {
		return u
	}
	return ""
}
