package handlers

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"scada-system/internal/api/middleware"
	"scada-system/internal/mqtt"
	"scada-system/internal/services"
)

type WaterHandler struct {
	water   *services.WaterService
	devices *services.DeviceService
	mqttCli *mqtt.Client
}

func NewWaterHandler(water *services.WaterService, devices *services.DeviceService, mqttCli *mqtt.Client) *WaterHandler {
	return &WaterHandler{water: water, devices: devices, mqttCli: mqttCli}
}

func (h *WaterHandler) GetAllSystems(w http.ResponseWriter, r *http.Request) {
	systems, err := h.water.GetAll(r.Context())
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, systems)
}

func (h *WaterHandler) GetSystem(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	system, err := h.water.GetStatus(r.Context(), id)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "water system not found"})
		return
	}
	writeJSON(w, http.StatusOK, system)
}

func (h *WaterHandler) ControlPump(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var req struct {
		Action string `json:"action"` // start, stop
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if req.Action != "start" && req.Action != "stop" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "action must be 'start' or 'stop'"})
		return
	}

	userID := middleware.GetUserID(r.Context())

	// Send MQTT command to device
	cmd := mqtt.CommandData{
		Action: req.Action + "_pump",
		Parameters: map[string]string{
			"system_id": id,
		},
		IssuedBy: userID,
		IssuedAt: time.Now(),
	}

	if err := h.mqttCli.PublishCommand("water", id, cmd); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to send command"})
		return
	}

	// Optimistically update local state
	status := "running"
	if req.Action == "stop" {
		status = "stopped"
	}
	h.water.SetPumpStatus(r.Context(), id, status)

	writeJSON(w, http.StatusOK, map[string]string{"status": "command sent", "action": req.Action})
}

func (h *WaterHandler) ControlValve(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var req struct {
		ValveID  string  `json:"valve_id"`
		Position float64 `json:"position"` // 0-100%
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if req.Position < 0 || req.Position > 100 {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "position must be 0-100"})
		return
	}

	userID := middleware.GetUserID(r.Context())

	cmd := mqtt.CommandData{
		Action: "set_valve",
		Parameters: map[string]string{
			"valve_id": req.ValveID,
			"position": json.Number(json.Number(string(rune(int(req.Position))))).String(),
		},
		IssuedBy: userID,
		IssuedAt: time.Now(),
	}

	if err := h.mqttCli.PublishCommand("water", id, cmd); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to send command"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "command sent"})
}

func (h *WaterHandler) SetMode(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var req struct {
		Mode string `json:"mode"` // auto, manual, maintenance
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if err := h.water.SetOperatingMode(r.Context(), id, req.Mode); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "mode updated", "mode": req.Mode})
}

func (h *WaterHandler) GetDevices(w http.ResponseWriter, r *http.Request) {
	devices, err := h.devices.GetAll(r.Context(), "water")
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, devices)
}
