package handlers

import (
	"net/http"
	"strconv"
	"time"

	"github.com/go-chi/chi/v5"
	"scada-system/internal/services"
)

type ReadingHandler struct {
	readings *services.ReadingService
}

func NewReadingHandler(readings *services.ReadingService) *ReadingHandler {
	return &ReadingHandler{readings: readings}
}

func (h *ReadingHandler) GetLatest(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	metric := r.URL.Query().Get("metric")

	if metric == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "metric parameter required"})
		return
	}

	reading, err := h.readings.GetLatest(r.Context(), deviceID, metric)
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "no reading found"})
		return
	}
	writeJSON(w, http.StatusOK, reading)
}

func (h *ReadingHandler) GetHistory(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	metric := r.URL.Query().Get("metric")
	fromStr := r.URL.Query().Get("from")
	toStr := r.URL.Query().Get("to")
	limitStr := r.URL.Query().Get("limit")

	if metric == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "metric parameter required"})
		return
	}

	from := time.Now().Add(-24 * time.Hour)
	to := time.Now()
	limit := 1000

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

	readings, err := h.readings.GetHistory(r.Context(), deviceID, metric, from, to, limit)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, readings)
}

func (h *ReadingHandler) GetAggregated(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	metric := r.URL.Query().Get("metric")
	interval := r.URL.Query().Get("interval")
	fromStr := r.URL.Query().Get("from")
	toStr := r.URL.Query().Get("to")

	if metric == "" || interval == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "metric and interval parameters required"})
		return
	}

	// Validate interval to prevent SQL injection
	validIntervals := map[string]bool{
		"1 minute": true, "5 minutes": true, "15 minutes": true,
		"30 minutes": true, "1 hour": true, "6 hours": true,
		"1 day": true, "1 week": true,
	}
	if !validIntervals[interval] {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid interval"})
		return
	}

	from := time.Now().Add(-24 * time.Hour)
	to := time.Now()

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

	results, err := h.readings.GetAggregated(r.Context(), deviceID, metric, interval, from, to)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, results)
}
