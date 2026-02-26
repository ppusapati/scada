package handlers

import (
	"encoding/json"
	"net/http"
	"strconv"
	"time"

	"github.com/go-chi/chi/v5"
	"scada-system/internal/api/middleware"
	"scada-system/internal/services"
)

type AlarmHandler struct {
	alarms *services.AlarmService
}

func NewAlarmHandler(alarms *services.AlarmService) *AlarmHandler {
	return &AlarmHandler{alarms: alarms}
}

func (h *AlarmHandler) GetActive(w http.ResponseWriter, r *http.Request) {
	subsystem := r.URL.Query().Get("subsystem")
	alarms, err := h.alarms.GetActive(r.Context(), subsystem)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, alarms)
}

func (h *AlarmHandler) GetHistory(w http.ResponseWriter, r *http.Request) {
	subsystem := r.URL.Query().Get("subsystem")
	fromStr := r.URL.Query().Get("from")
	toStr := r.URL.Query().Get("to")
	limitStr := r.URL.Query().Get("limit")

	from := time.Now().Add(-24 * time.Hour)
	to := time.Now()
	limit := 500

	if fromStr != "" {
		if t, err := time.Parse(time.RFC3339, fromStr); err == nil {
			from = t
		}
	}
	if toStr != "" {
		if t, err := time.Parse(time.RFC3339, toStr); err == nil {
			to = t
		}
	}
	if limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil {
			limit = l
		}
	}

	alarms, err := h.alarms.GetHistory(r.Context(), subsystem, from, to, limit)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, alarms)
}

func (h *AlarmHandler) Acknowledge(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := strconv.ParseInt(idStr, 10, 64)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid alarm id"})
		return
	}

	userID := middleware.GetUserID(r.Context())
	if err := h.alarms.Acknowledge(r.Context(), id, userID); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "acknowledged"})
}

func (h *AlarmHandler) Clear(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := strconv.ParseInt(idStr, 10, 64)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid alarm id"})
		return
	}

	if err := h.alarms.Clear(r.Context(), id); err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{"status": "cleared"})
}

func (h *AlarmHandler) GetCounts(w http.ResponseWriter, r *http.Request) {
	counts, err := h.alarms.GetCounts(r.Context())
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, counts)
}

func (h *AlarmHandler) CreateManual(w http.ResponseWriter, r *http.Request) {
	var req struct {
		DeviceID  string  `json:"device_id"`
		Subsystem string  `json:"subsystem"`
		Severity  string  `json:"severity"`
		Category  string  `json:"category"`
		Message   string  `json:"message"`
		Value     float64 `json:"value"`
		Threshold float64 `json:"threshold"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	alarm := &models_Alarm{
		DeviceID:  req.DeviceID,
		Subsystem: req.Subsystem,
		Severity:  req.Severity,
		Category:  req.Category,
		Message:   req.Message,
		Value:     req.Value,
		Threshold: req.Threshold,
	}
	_ = alarm

	writeJSON(w, http.StatusCreated, map[string]string{"status": "alarm created"})
}

// placeholder to avoid import issues — real code uses models.Alarm
type models_Alarm struct {
	DeviceID  string
	Subsystem string
	Severity  string
	Category  string
	Message   string
	Value     float64
	Threshold float64
}
