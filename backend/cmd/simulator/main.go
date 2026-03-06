package main

import (
	"encoding/json"
	"fmt"
	"math"
	"math/rand"
	"os"
	"os/signal"
	"syscall"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"
)

// Simulator generates realistic SCADA sensor data over MQTT
// Use this for development and testing without real hardware

func main() {
	fmt.Println("╔══════════════════════════════════════════╗")
	fmt.Println("║  SCADA Data Simulator                    ║")
	fmt.Println("║  Generating test data over MQTT          ║")
	fmt.Println("╚══════════════════════════════════════════╝")

	broker := getEnv("MQTT_BROKER", "localhost")
	port := getEnv("MQTT_PORT", "1883")

	opts := pahomqtt.NewClientOptions().
		AddBroker(fmt.Sprintf("tcp://%s:%s", broker, port)).
		SetClientID("scada-simulator").
		SetAutoReconnect(true)

	client := pahomqtt.NewClient(opts)
	if token := client.Connect(); token.Wait() && token.Error() != nil {
		fmt.Printf("Failed to connect: %v\n", token.Error())
		os.Exit(1)
	}
	fmt.Printf("Connected to MQTT broker at %s:%s\n\n", broker, port)

	waterDeviceID := "sim-water-001"
	solarDeviceID := "sim-solar-001"

	// Publish device online status
	publishStatus(client, "water", waterDeviceID, "online")
	publishStatus(client, "solar", solarDeviceID, "online")

	// State for simulation continuity
	waterState := &WaterState{
		TankLevel:     65.0,
		FlowIn:        45.0,
		FlowOut:       42.0,
		Pressure:      3.5,
		pH:            7.2,
		Turbidity:     1.5,
		Chlorine:      0.8,
		PumpRunning:   true,
	}

	solarState := &SolarState{
		HourOfDay:    12.0,
		Irradiance:   800.0,
		PanelTemp:    45.0,
		GridVoltage:  230.0,
		GridFreq:     50.0,
		OutputKW:     3.2,
		DailyKWH:     0.0,
		Efficiency:   19.5,
	}

	// Subscribe to commands
	client.Subscribe(fmt.Sprintf("scada/water/%s/command", waterDeviceID), 1, func(c pahomqtt.Client, m pahomqtt.Message) {
		var cmd map[string]interface{}
		if err := json.Unmarshal(m.Payload(), &cmd); err != nil {
			fmt.Printf("[CMD] Failed to parse water command: %v\n", err)
			return
		}
		action := fmt.Sprintf("%v", cmd["action"])
		fmt.Printf("[CMD] Water: %s\n", action)
		switch action {
		case "start_pump":
			waterState.PumpRunning = true
		case "stop_pump":
			waterState.PumpRunning = false
		}
	})

	client.Subscribe(fmt.Sprintf("scada/solar/%s/command", solarDeviceID), 1, func(c pahomqtt.Client, m pahomqtt.Message) {
		var cmd map[string]interface{}
		if err := json.Unmarshal(m.Payload(), &cmd); err != nil {
			fmt.Printf("[CMD] Failed to parse solar command: %v\n", err)
			return
		}
		action := fmt.Sprintf("%v", cmd["action"])
		fmt.Printf("[CMD] Solar: %s\n", action)
	})

	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	fmt.Println("Publishing simulated data every 5 seconds...")
	fmt.Println("Press Ctrl+C to stop\n")

	for {
		select {
		case <-ticker.C:
			// Water telemetry
			waterMetrics := waterState.Step()
			publishTelemetry(client, "water", waterDeviceID, waterMetrics)
			fmt.Printf("[Water] Tank: %.1f%% | Flow In: %.1f L/min | pH: %.2f | Pressure: %.2f bar\n",
				waterMetrics["tank_level"], waterMetrics["flow_rate_in"],
				waterMetrics["ph_level"], waterMetrics["pressure"])

			// Solar telemetry
			solarMetrics := solarState.Step()
			publishTelemetry(client, "solar", solarDeviceID, solarMetrics)
			fmt.Printf("[Solar] Irradiance: %.0f W/m² | Output: %.2f kW | Temp: %.1f°C | Grid: %.1fV\n",
				solarMetrics["irradiance"], solarMetrics["current_output_kw"],
				solarMetrics["panel_temperature"], solarMetrics["grid_voltage"])
			fmt.Println()

		case <-sigChan:
			fmt.Println("\nShutting down simulator...")
			publishStatus(client, "water", waterDeviceID, "offline")
			publishStatus(client, "solar", solarDeviceID, "offline")
			client.Disconnect(1000)
			return
		}
	}
}

type WaterState struct {
	TankLevel   float64
	FlowIn      float64
	FlowOut     float64
	Pressure    float64
	pH          float64
	Turbidity   float64
	Chlorine    float64
	PumpRunning bool
}

func (w *WaterState) Step() map[string]float64 {
	netFlow := w.FlowIn - w.FlowOut
	w.TankLevel += netFlow * 0.001
	w.TankLevel = clamp(w.TankLevel, 0, 100)

	if w.PumpRunning {
		w.FlowIn = 45 + randRange(-3, 3)
		w.FlowOut = 42 + randRange(-2, 2)
		w.Pressure = 3.5 + randRange(-0.3, 0.3)
	} else {
		w.FlowIn = randRange(0, 2)
		w.FlowOut = randRange(0, 1)
		w.Pressure = 1.0 + randRange(-0.1, 0.1)
	}

	w.pH += randRange(-0.05, 0.05)
	w.pH = clamp(w.pH, 5.0, 9.0)

	if rand.Intn(100) < 5 {
		w.Turbidity = randRange(2, 5)
	} else {
		w.Turbidity = clamp(1.5+randRange(-0.5, 0.5), 0.1, 10)
	}

	w.Chlorine += randRange(-0.02, 0.02)
	w.Chlorine = clamp(w.Chlorine, 0, 3)

	return map[string]float64{
		"tank_level":     w.TankLevel,
		"flow_rate_in":   w.FlowIn,
		"flow_rate_out":  w.FlowOut,
		"pressure":       w.Pressure,
		"ph_level":       w.pH,
		"turbidity":      w.Turbidity,
		"chlorine_level": w.Chlorine,
		"temperature":    22 + randRange(-1, 1),
	}
}

type SolarState struct {
	HourOfDay  float64
	Irradiance float64
	PanelTemp  float64
	GridVoltage float64
	GridFreq   float64
	OutputKW   float64
	DailyKWH   float64
	Efficiency float64
}

func (s *SolarState) Step() map[string]float64 {
	s.HourOfDay += 0.05
	if s.HourOfDay > 24 {
		s.HourOfDay = 0
		s.DailyKWH = 0
	}

	// Irradiance bell curve
	solarAngle := math.Cos((s.HourOfDay - 12.0) / 6.0 * math.Pi / 2)
	if s.HourOfDay > 6 && s.HourOfDay < 18 {
		cloudFactor := 0.9 + randRange(0, 0.1)
		if rand.Intn(100) < 10 {
			cloudFactor = randRange(0.3, 0.7)
		}
		s.Irradiance = math.Max(0, 1000*math.Max(0, solarAngle)*cloudFactor+randRange(-20, 20))
	} else {
		s.Irradiance = 0
	}

	ambient := 25 + randRange(-2, 2)
	s.PanelTemp = ambient + (s.Irradiance/1000)*30 + randRange(-1, 1)

	if s.Irradiance > 50 {
		tempFactor := 1.0 - math.Max(0, (s.PanelTemp-25))*0.004
		irrFactor := s.Irradiance / 1000
		s.OutputKW = math.Max(0, 380*math.Pow(irrFactor, 0.3)*tempFactor*10*irrFactor*tempFactor/1000*0.96)
		if s.Irradiance > 100 {
			s.Efficiency = (s.OutputKW / (s.Irradiance / 1000 * 20)) * 100
		}
		s.DailyKWH += s.OutputKW * (0.05 / 60)
	} else {
		s.OutputKW = 0
		s.Efficiency = 0
	}

	s.GridVoltage = 230 + randRange(-3, 3)
	s.GridFreq = 50 + randRange(-0.05, 0.05)

	return map[string]float64{
		"irradiance":        s.Irradiance,
		"panel_temperature":  s.PanelTemp,
		"current_output_kw":  s.OutputKW,
		"grid_voltage":       s.GridVoltage,
		"grid_frequency":     s.GridFreq,
		"daily_energy_kwh":   s.DailyKWH,
		"efficiency":         s.Efficiency,
	}
}

func publishTelemetry(client pahomqtt.Client, subsystem, deviceID string, metrics map[string]float64) {
	payload, err := json.Marshal(map[string]interface{}{
		"device_id": deviceID,
		"timestamp": time.Now().Format(time.RFC3339),
		"metrics":   metrics,
		"quality":   0,
	})
	if err != nil {
		fmt.Printf("[Sim] Failed to marshal telemetry: %v\n", err)
		return
	}
	topic := fmt.Sprintf("scada/%s/%s/telemetry", subsystem, deviceID)
	client.Publish(topic, 1, false, payload)
}

func publishStatus(client pahomqtt.Client, subsystem, deviceID, status string) {
	payload, err := json.Marshal(map[string]interface{}{
		"device_id": deviceID,
		"status":    status,
		"timestamp": time.Now().Format(time.RFC3339),
	})
	if err != nil {
		fmt.Printf("[Sim] Failed to marshal status: %v\n", err)
		return
	}
	topic := fmt.Sprintf("scada/%s/%s/status", subsystem, deviceID)
	client.Publish(topic, 1, true, payload)
}

func randRange(min, max float64) float64 {
	return min + rand.Float64()*(max-min)
}

func clamp(v, min, max float64) float64 {
	if v < min { return min }
	if v > max { return max }
	return v
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
