package services

import (
	"context"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
)

type DeviceService struct {
	pool *pgxpool.Pool
}

func NewDeviceService(pool *pgxpool.Pool) *DeviceService {
	return &DeviceService{pool: pool}
}

func (s *DeviceService) GetAll(ctx context.Context, subsystem string) ([]models.Device, error) {
	query := `SELECT id, name, device_type, location, subsystem, status, last_seen, metadata, created_at, updated_at
		FROM devices WHERE ($1 = '' OR subsystem = $1) ORDER BY name`

	rows, err := s.pool.Query(ctx, query, subsystem)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var devices []models.Device
	for rows.Next() {
		var d models.Device
		err := rows.Scan(&d.ID, &d.Name, &d.DeviceType, &d.Location, &d.Subsystem,
			&d.Status, &d.LastSeen, &d.Metadata, &d.CreatedAt, &d.UpdatedAt)
		if err != nil {
			return nil, err
		}
		devices = append(devices, d)
	}
	return devices, nil
}

func (s *DeviceService) GetByID(ctx context.Context, id string) (*models.Device, error) {
	var d models.Device
	err := s.pool.QueryRow(ctx,
		`SELECT id, name, device_type, location, subsystem, status, last_seen, metadata, created_at, updated_at
		FROM devices WHERE id = $1`, id).
		Scan(&d.ID, &d.Name, &d.DeviceType, &d.Location, &d.Subsystem,
			&d.Status, &d.LastSeen, &d.Metadata, &d.CreatedAt, &d.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &d, nil
}

func (s *DeviceService) Create(ctx context.Context, d *models.Device) error {
	return s.pool.QueryRow(ctx,
		`INSERT INTO devices (name, device_type, location, subsystem, status, metadata)
		VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, created_at, updated_at`,
		d.Name, d.DeviceType, d.Location, d.Subsystem, d.Status, d.Metadata).
		Scan(&d.ID, &d.CreatedAt, &d.UpdatedAt)
}

func (s *DeviceService) UpdateStatus(ctx context.Context, id, status string) error {
	_, err := s.pool.Exec(ctx,
		`UPDATE devices SET status = $1, last_seen = $2, updated_at = $2 WHERE id = $3`,
		status, time.Now(), id)
	return err
}

func (s *DeviceService) Delete(ctx context.Context, id string) error {
	_, err := s.pool.Exec(ctx, `DELETE FROM devices WHERE id = $1`, id)
	return err
}
