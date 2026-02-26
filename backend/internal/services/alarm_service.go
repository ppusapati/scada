package services

import (
	"context"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
)

type AlarmService struct {
	pool *pgxpool.Pool
}

func NewAlarmService(pool *pgxpool.Pool) *AlarmService {
	return &AlarmService{pool: pool}
}

func (s *AlarmService) Create(ctx context.Context, a *models.Alarm) error {
	return s.pool.QueryRow(ctx,
		`INSERT INTO alarms (device_id, subsystem, severity, category, message, value, threshold, status)
		VALUES ($1, $2, $3, $4, $5, $6, $7, 'active') RETURNING id, occurred_at`,
		a.DeviceID, a.Subsystem, a.Severity, a.Category, a.Message, a.Value, a.Threshold).
		Scan(&a.ID, &a.OccurredAt)
}

func (s *AlarmService) GetActive(ctx context.Context, subsystem string) ([]models.Alarm, error) {
	query := `SELECT id, device_id, subsystem, severity, category, message, value, threshold,
		status, occurred_at, acked_at, acked_by, cleared_at
		FROM alarms WHERE status != 'cleared' AND ($1 = '' OR subsystem = $1)
		ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END, occurred_at DESC`

	rows, err := s.pool.Query(ctx, query, subsystem)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var alarms []models.Alarm
	for rows.Next() {
		var a models.Alarm
		if err := rows.Scan(&a.ID, &a.DeviceID, &a.Subsystem, &a.Severity, &a.Category,
			&a.Message, &a.Value, &a.Threshold, &a.Status, &a.OccurredAt,
			&a.AckedAt, &a.AckedBy, &a.ClearedAt); err != nil {
			return nil, err
		}
		alarms = append(alarms, a)
	}
	return alarms, nil
}

func (s *AlarmService) GetHistory(ctx context.Context, subsystem string, from, to time.Time, limit int) ([]models.Alarm, error) {
	if limit <= 0 || limit > 10000 {
		limit = 500
	}
	rows, err := s.pool.Query(ctx,
		`SELECT id, device_id, subsystem, severity, category, message, value, threshold,
			status, occurred_at, acked_at, acked_by, cleared_at
		FROM alarms WHERE ($1 = '' OR subsystem = $1) AND occurred_at BETWEEN $2 AND $3
		ORDER BY occurred_at DESC LIMIT $4`,
		subsystem, from, to, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var alarms []models.Alarm
	for rows.Next() {
		var a models.Alarm
		if err := rows.Scan(&a.ID, &a.DeviceID, &a.Subsystem, &a.Severity, &a.Category,
			&a.Message, &a.Value, &a.Threshold, &a.Status, &a.OccurredAt,
			&a.AckedAt, &a.AckedBy, &a.ClearedAt); err != nil {
			return nil, err
		}
		alarms = append(alarms, a)
	}
	return alarms, nil
}

func (s *AlarmService) Acknowledge(ctx context.Context, alarmID int64, userID string) error {
	now := time.Now()
	_, err := s.pool.Exec(ctx,
		`UPDATE alarms SET status = 'acknowledged', acked_at = $1, acked_by = $2 WHERE id = $3 AND status = 'active'`,
		now, userID, alarmID)
	return err
}

func (s *AlarmService) Clear(ctx context.Context, alarmID int64) error {
	now := time.Now()
	_, err := s.pool.Exec(ctx,
		`UPDATE alarms SET status = 'cleared', cleared_at = $1 WHERE id = $2`,
		now, alarmID)
	return err
}

func (s *AlarmService) GetCounts(ctx context.Context) (map[string]int, error) {
	rows, err := s.pool.Query(ctx,
		`SELECT severity, COUNT(*) FROM alarms WHERE status = 'active' GROUP BY severity`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	counts := map[string]int{"critical": 0, "warning": 0, "info": 0}
	for rows.Next() {
		var severity string
		var count int
		if err := rows.Scan(&severity, &count); err != nil {
			return nil, err
		}
		counts[severity] = count
	}
	return counts, nil
}
