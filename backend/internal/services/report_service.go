package services

import (
	"context"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

// ReportService generates operational reports and analytics
type ReportService struct {
	pool *pgxpool.Pool
}

func NewReportService(pool *pgxpool.Pool) *ReportService {
	return &ReportService{pool: pool}
}

// DailySummary holds a daily operational summary
type DailySummary struct {
	Date              string                   `json:"date"`
	WaterMetrics      map[string]MetricSummary `json:"water_metrics"`
	SolarMetrics      map[string]MetricSummary `json:"solar_metrics"`
	AlarmSummary      AlarmSummaryReport       `json:"alarm_summary"`
	UptimePercentage  float64                  `json:"uptime_percentage"`
}

type MetricSummary struct {
	Avg     float64 `json:"avg"`
	Min     float64 `json:"min"`
	Max     float64 `json:"max"`
	StdDev  float64 `json:"std_dev"`
	Samples int64   `json:"samples"`
}

type AlarmSummaryReport struct {
	TotalAlarms    int            `json:"total_alarms"`
	BySeverity     map[string]int `json:"by_severity"`
	BySubsystem    map[string]int `json:"by_subsystem"`
	AvgResponseMin float64        `json:"avg_response_minutes"`
}

// GetDailySummary generates a summary report for a given date
func (s *ReportService) GetDailySummary(ctx context.Context, date time.Time) (*DailySummary, error) {
	startOfDay := time.Date(date.Year(), date.Month(), date.Day(), 0, 0, 0, 0, date.Location())
	endOfDay := startOfDay.Add(24 * time.Hour)

	summary := &DailySummary{
		Date:         startOfDay.Format("2006-01-02"),
		WaterMetrics: make(map[string]MetricSummary),
		SolarMetrics: make(map[string]MetricSummary),
	}

	// Fetch aggregated metrics for water
	waterMetrics := []string{"tank_level", "flow_rate_in", "flow_rate_out", "pressure", "ph_level", "turbidity", "chlorine_level"}
	for _, metric := range waterMetrics {
		ms, err := s.getMetricSummary(ctx, "water", metric, startOfDay, endOfDay)
		if err == nil && ms != nil {
			summary.WaterMetrics[metric] = *ms
		}
	}

	// Fetch aggregated metrics for solar
	solarMetrics := []string{"irradiance", "current_output_kw", "panel_temperature", "grid_voltage", "grid_frequency", "daily_energy_kwh"}
	for _, metric := range solarMetrics {
		ms, err := s.getMetricSummary(ctx, "solar", metric, startOfDay, endOfDay)
		if err == nil && ms != nil {
			summary.SolarMetrics[metric] = *ms
		}
	}

	// Alarm summary
	alarmSummary, err := s.getAlarmSummary(ctx, startOfDay, endOfDay)
	if err == nil {
		summary.AlarmSummary = *alarmSummary
	}

	// Uptime (based on device online time)
	uptime, err := s.calculateUptime(ctx, startOfDay, endOfDay)
	if err == nil {
		summary.UptimePercentage = uptime
	}

	return summary, nil
}

func (s *ReportService) getMetricSummary(ctx context.Context, subsystem, metric string, from, to time.Time) (*MetricSummary, error) {
	var ms MetricSummary
	err := s.pool.QueryRow(ctx,
		`SELECT COALESCE(AVG(sr.value), 0), COALESCE(MIN(sr.value), 0), COALESCE(MAX(sr.value), 0),
			COALESCE(STDDEV(sr.value), 0), COUNT(*)
		FROM sensor_readings sr
		JOIN devices d ON sr.device_id = d.id::text
		WHERE d.subsystem = $1 AND sr.metric = $2 AND sr.timestamp BETWEEN $3 AND $4`,
		subsystem, metric, from, to).
		Scan(&ms.Avg, &ms.Min, &ms.Max, &ms.StdDev, &ms.Samples)
	if err != nil {
		return nil, err
	}
	return &ms, nil
}

func (s *ReportService) getAlarmSummary(ctx context.Context, from, to time.Time) (*AlarmSummaryReport, error) {
	summary := &AlarmSummaryReport{
		BySeverity:  make(map[string]int),
		BySubsystem: make(map[string]int),
	}

	// Total and by severity
	rows, err := s.pool.Query(ctx,
		`SELECT severity, COUNT(*) FROM alarms WHERE occurred_at BETWEEN $1 AND $2 GROUP BY severity`,
		from, to)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var severity string
		var count int
		if err := rows.Scan(&severity, &count); err != nil {
			return nil, err
		}
		summary.BySeverity[severity] = count
		summary.TotalAlarms += count
	}

	// By subsystem
	rows2, err := s.pool.Query(ctx,
		`SELECT subsystem, COUNT(*) FROM alarms WHERE occurred_at BETWEEN $1 AND $2 GROUP BY subsystem`,
		from, to)
	if err != nil {
		return nil, err
	}
	defer rows2.Close()

	for rows2.Next() {
		var subsystem string
		var count int
		if err := rows2.Scan(&subsystem, &count); err != nil {
			return nil, err
		}
		summary.BySubsystem[subsystem] = count
	}

	// Average response time (time to acknowledge)
	var avgResponse *float64
	s.pool.QueryRow(ctx,
		`SELECT AVG(EXTRACT(EPOCH FROM (acked_at - occurred_at)) / 60.0)
		FROM alarms WHERE occurred_at BETWEEN $1 AND $2 AND acked_at IS NOT NULL`,
		from, to).Scan(&avgResponse)
	if avgResponse != nil {
		summary.AvgResponseMin = *avgResponse
	}

	return summary, nil
}

func (s *ReportService) calculateUptime(ctx context.Context, from, to time.Time) (float64, error) {
	var totalDevices, onlineDevices int64
	err := s.pool.QueryRow(ctx,
		`SELECT COUNT(*), COUNT(*) FILTER (WHERE status = 'online' OR last_seen > $1)
		FROM devices`,
		from).Scan(&totalDevices, &onlineDevices)
	if err != nil || totalDevices == 0 {
		return 100.0, err
	}
	return float64(onlineDevices) / float64(totalDevices) * 100.0, nil
}

// GetEnergyReport generates a solar energy production report
func (s *ReportService) GetEnergyReport(ctx context.Context, from, to time.Time) ([]map[string]interface{}, error) {
	rows, err := s.pool.Query(ctx,
		`SELECT time_bucket('1 day', timestamp) as day,
			SUM(value) as total_kwh,
			AVG(value) as avg_kwh,
			MAX(value) as peak_kwh
		FROM sensor_readings
		WHERE metric = 'daily_energy_kwh' AND timestamp BETWEEN $1 AND $2
		GROUP BY day ORDER BY day`, from, to)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []map[string]interface{}
	for rows.Next() {
		var day time.Time
		var total, avg, peak float64
		if err := rows.Scan(&day, &total, &avg, &peak); err != nil {
			return nil, err
		}
		results = append(results, map[string]interface{}{
			"date":      day.Format("2006-01-02"),
			"total_kwh": fmt.Sprintf("%.2f", total),
			"avg_kwh":   fmt.Sprintf("%.2f", avg),
			"peak_kwh":  fmt.Sprintf("%.2f", peak),
		})
	}
	return results, nil
}

// GetWaterQualityReport generates a water quality compliance report
func (s *ReportService) GetWaterQualityReport(ctx context.Context, from, to time.Time) (map[string]interface{}, error) {
	report := make(map[string]interface{})

	// pH compliance (6.5-8.5)
	var totalPH, compliantPH int64
	s.pool.QueryRow(ctx,
		`SELECT COUNT(*), COUNT(*) FILTER (WHERE value BETWEEN 6.5 AND 8.5)
		FROM sensor_readings WHERE metric = 'ph_level' AND timestamp BETWEEN $1 AND $2`,
		from, to).Scan(&totalPH, &compliantPH)

	if totalPH > 0 {
		report["ph_compliance_pct"] = float64(compliantPH) / float64(totalPH) * 100
	}

	// Turbidity compliance (< 4 NTU)
	var totalTurb, compliantTurb int64
	s.pool.QueryRow(ctx,
		`SELECT COUNT(*), COUNT(*) FILTER (WHERE value < 4.0)
		FROM sensor_readings WHERE metric = 'turbidity' AND timestamp BETWEEN $1 AND $2`,
		from, to).Scan(&totalTurb, &compliantTurb)

	if totalTurb > 0 {
		report["turbidity_compliance_pct"] = float64(compliantTurb) / float64(totalTurb) * 100
	}

	// Chlorine compliance (0.2-1.5 mg/L)
	var totalChl, compliantChl int64
	s.pool.QueryRow(ctx,
		`SELECT COUNT(*), COUNT(*) FILTER (WHERE value BETWEEN 0.2 AND 1.5)
		FROM sensor_readings WHERE metric = 'chlorine_level' AND timestamp BETWEEN $1 AND $2`,
		from, to).Scan(&totalChl, &compliantChl)

	if totalChl > 0 {
		report["chlorine_compliance_pct"] = float64(compliantChl) / float64(totalChl) * 100
	}

	report["period_from"] = from.Format("2006-01-02")
	report["period_to"] = to.Format("2006-01-02")

	return report, nil
}
