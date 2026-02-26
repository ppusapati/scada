package services

import (
	"context"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
)

type ReadingService struct {
	pool *pgxpool.Pool
}

func NewReadingService(pool *pgxpool.Pool) *ReadingService {
	return &ReadingService{pool: pool}
}

func (s *ReadingService) Insert(ctx context.Context, r *models.SensorReading) error {
	_, err := s.pool.Exec(ctx,
		`INSERT INTO sensor_readings (device_id, timestamp, metric, value, unit, quality)
		VALUES ($1, $2, $3, $4, $5, $6)`,
		r.DeviceID, r.Timestamp, r.Metric, r.Value, r.Unit, r.Quality)
	return err
}

func (s *ReadingService) InsertBatch(ctx context.Context, readings []models.SensorReading) error {
	batch := &pgxpool.Pool{}
	_ = batch // use pgx batch for production
	for _, r := range readings {
		if err := s.Insert(ctx, &r); err != nil {
			return err
		}
	}
	return nil
}

func (s *ReadingService) GetLatest(ctx context.Context, deviceID, metric string) (*models.SensorReading, error) {
	var r models.SensorReading
	err := s.pool.QueryRow(ctx,
		`SELECT id, device_id, timestamp, metric, value, unit, quality
		FROM sensor_readings WHERE device_id = $1 AND metric = $2
		ORDER BY timestamp DESC LIMIT 1`, deviceID, metric).
		Scan(&r.ID, &r.DeviceID, &r.Timestamp, &r.Metric, &r.Value, &r.Unit, &r.Quality)
	if err != nil {
		return nil, err
	}
	return &r, nil
}

func (s *ReadingService) GetHistory(ctx context.Context, deviceID, metric string, from, to time.Time, limit int) ([]models.SensorReading, error) {
	if limit <= 0 || limit > 10000 {
		limit = 1000
	}
	rows, err := s.pool.Query(ctx,
		`SELECT id, device_id, timestamp, metric, value, unit, quality
		FROM sensor_readings
		WHERE device_id = $1 AND metric = $2 AND timestamp BETWEEN $3 AND $4
		ORDER BY timestamp DESC LIMIT $5`,
		deviceID, metric, from, to, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var readings []models.SensorReading
	for rows.Next() {
		var r models.SensorReading
		if err := rows.Scan(&r.ID, &r.DeviceID, &r.Timestamp, &r.Metric, &r.Value, &r.Unit, &r.Quality); err != nil {
			return nil, err
		}
		readings = append(readings, r)
	}
	return readings, nil
}

// GetAggregated returns aggregated readings (avg, min, max) bucketed by time interval
func (s *ReadingService) GetAggregated(ctx context.Context, deviceID, metric, interval string, from, to time.Time) ([]map[string]interface{}, error) {
	query := `SELECT time_bucket($1::interval, timestamp) AS bucket,
		AVG(value) as avg_value,
		MIN(value) as min_value,
		MAX(value) as max_value,
		COUNT(*) as sample_count
		FROM sensor_readings
		WHERE device_id = $2 AND metric = $3 AND timestamp BETWEEN $4 AND $5
		GROUP BY bucket ORDER BY bucket`

	rows, err := s.pool.Query(ctx, query, interval, deviceID, metric, from, to)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []map[string]interface{}
	for rows.Next() {
		var bucket time.Time
		var avg, min, max float64
		var count int64
		if err := rows.Scan(&bucket, &avg, &min, &max, &count); err != nil {
			return nil, err
		}
		results = append(results, map[string]interface{}{
			"timestamp":    bucket,
			"avg_value":    avg,
			"min_value":    min,
			"max_value":    max,
			"sample_count": count,
		})
	}
	return results, nil
}
