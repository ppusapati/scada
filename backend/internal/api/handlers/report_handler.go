package handlers

import (
	"net/http"
	"time"

	"scada-system/internal/services"
)

type ReportHandler struct {
	reports *services.ReportService
}

func NewReportHandler(reports *services.ReportService) *ReportHandler {
	return &ReportHandler{reports: reports}
}

func (h *ReportHandler) GetDailySummary(w http.ResponseWriter, r *http.Request) {
	dateStr := r.URL.Query().Get("date")

	var date time.Time
	if dateStr != "" {
		var err error
		date, err = time.Parse("2006-01-02", dateStr)
		if err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid date format, use YYYY-MM-DD"})
			return
		}
	} else {
		date = time.Now()
	}

	summary, err := h.reports.GetDailySummary(r.Context(), date)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, summary)
}

func (h *ReportHandler) GetEnergyReport(w http.ResponseWriter, r *http.Request) {
	from, to := parseTimeRange(r)

	report, err := h.reports.GetEnergyReport(r.Context(), from, to)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, report)
}

func (h *ReportHandler) GetWaterQualityReport(w http.ResponseWriter, r *http.Request) {
	from, to := parseTimeRange(r)

	report, err := h.reports.GetWaterQualityReport(r.Context(), from, to)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, report)
}

func parseTimeRange(r *http.Request) (time.Time, time.Time) {
	fromStr := r.URL.Query().Get("from")
	toStr := r.URL.Query().Get("to")

	from := time.Now().Add(-7 * 24 * time.Hour)
	to := time.Now()

	if fromStr != "" {
		if t, err := time.Parse("2006-01-02", fromStr); err == nil {
			from = t
		} else if t, err := time.Parse(time.RFC3339, fromStr); err == nil {
			from = t
		}
	}
	if toStr != "" {
		if t, err := time.Parse("2006-01-02", toStr); err == nil {
			to = t.Add(24 * time.Hour) // End of day
		} else if t, err := time.Parse(time.RFC3339, toStr); err == nil {
			to = t
		}
	}

	return from, to
}
