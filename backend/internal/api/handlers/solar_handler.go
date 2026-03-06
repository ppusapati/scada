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

type SolarHandler struct {
	solar   *services.SolarService
	devices *services.DeviceService
	mqttCli *mqtt.Client
}

func NewSolarHandler(solar *services.SolarService, devices *services.DeviceService, mqttCli *mqtt.Client) *SolarHandler {
	return &SolarHandler{solar: solar, devices: devices, mqttCli: mqttCli}
}

func (h *SolarHandler) GetAllSystems(w http.ResponseWriter, r *http.Request) {
	systems, err := h.solar.GetAll(r.Context())
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, systems)
}

func (h *SolarHandler) GetSystem(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	system, err := h.solar.GetStatus(r.Context(), id)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "solar system not found"})
		return
	}
	writeJSON(w, http.StatusOK, system)
}

func (h *SolarHandler) ControlInverter(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var req struct {
		Action string `json:"action"` // enable, disable, reset
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if req.Action != "enable" && req.Action != "disable" && req.Action != "reset" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "action must be 'enable', 'disable', or 'reset'"})
		return
	}

	userID := middleware.GetUserID(r.Context())

	cmd := mqtt.CommandData{
		Action: req.Action + "_inverter",
		Parameters: map[string]string{
			"system_id": id,
		},
		IssuedBy: userID,
		IssuedAt: time.Now(),
	}

	if err := h.mqttCli.PublishCommand("solar", id, cmd); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to send command"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "command sent", "action": req.Action})
}

func (h *SolarHandler) SetMode(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var req struct {
		Mode string `json:"mode"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if req.Mode != "auto" && req.Mode != "manual" && req.Mode != "maintenance" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "mode must be 'auto', 'manual', or 'maintenance'"})
		return
	}

	if err := h.solar.SetOperatingMode(r.Context(), id, req.Mode); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to update mode"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "mode updated", "mode": req.Mode})
}

func (h *SolarHandler) GetDevices(w http.ResponseWriter, r *http.Request) {
	devices, err := h.devices.GetAll(r.Context(), "solar")
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, devices)
}

func (h *SolarHandler) GetDailyProduction(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	system, err := h.solar.GetStatus(r.Context(), id)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "system not found"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"system_id":        system.ID,
		"daily_energy_kwh": system.DailyEnergyKWH,
		"total_energy_mwh": system.TotalEnergyMWH,
		"current_output_kw": system.CurrentOutputKW,
		"efficiency":       system.Efficiency,
	})
}
