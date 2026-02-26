package services

import (
	"context"
	"fmt"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
)

type WaterService struct {
	pool *pgxpool.Pool
}

func NewWaterService(pool *pgxpool.Pool) *WaterService {
	return &WaterService{pool: pool}
}

func (s *WaterService) GetStatus(ctx context.Context, id string) (*models.WaterSystem, error) {
	var w models.WaterSystem
	err := s.pool.QueryRow(ctx,
		`SELECT id, name, tank_level, flow_rate_in, flow_rate_out, pressure,
			ph_level, turbidity, chlorine_level, pump_status, valve_positions,
			operating_mode, updated_at
		FROM water_systems WHERE id = $1`, id).
		Scan(&w.ID, &w.Name, &w.TankLevel, &w.FlowRateIn, &w.FlowRateOut,
			&w.Pressure, &w.PHLevel, &w.Turbidity, &w.ChlorineLevel,
			&w.PumpStatus, &w.ValvePositions, &w.OperatingMode, &w.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &w, nil
}

func (s *WaterService) GetAll(ctx context.Context) ([]models.WaterSystem, error) {
	rows, err := s.pool.Query(ctx,
		`SELECT id, name, tank_level, flow_rate_in, flow_rate_out, pressure,
			ph_level, turbidity, chlorine_level, pump_status, valve_positions,
			operating_mode, updated_at
		FROM water_systems ORDER BY name`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var systems []models.WaterSystem
	for rows.Next() {
		var w models.WaterSystem
		if err := rows.Scan(&w.ID, &w.Name, &w.TankLevel, &w.FlowRateIn, &w.FlowRateOut,
			&w.Pressure, &w.PHLevel, &w.Turbidity, &w.ChlorineLevel,
			&w.PumpStatus, &w.ValvePositions, &w.OperatingMode, &w.UpdatedAt); err != nil {
			return nil, err
		}
		systems = append(systems, w)
	}
	return systems, nil
}

func (s *WaterService) UpdateFromMQTT(ctx context.Context, id string, updates map[string]interface{}) error {
	// Dynamic update based on received MQTT fields
	query := `UPDATE water_systems SET updated_at = NOW()`
	args := []interface{}{}
	argIdx := 1

	fieldMap := map[string]string{
		"tank_level":     "tank_level",
		"flow_rate_in":   "flow_rate_in",
		"flow_rate_out":  "flow_rate_out",
		"pressure":       "pressure",
		"ph_level":       "ph_level",
		"turbidity":      "turbidity",
		"chlorine_level": "chlorine_level",
		"pump_status":    "pump_status",
	}

	for jsonKey, dbCol := range fieldMap {
		if val, ok := updates[jsonKey]; ok {
			query += fmt.Sprintf(", %s = $%d", dbCol, argIdx)
			args = append(args, val)
			argIdx++
		}
	}

	query += fmt.Sprintf(" WHERE id = $%d", argIdx)
	args = append(args, id)

	_, err := s.pool.Exec(ctx, query, args...)
	return err
}

func (s *WaterService) SetOperatingMode(ctx context.Context, id, mode string) error {
	_, err := s.pool.Exec(ctx,
		`UPDATE water_systems SET operating_mode = $1, updated_at = NOW() WHERE id = $2`,
		mode, id)
	return err
}

func (s *WaterService) SetPumpStatus(ctx context.Context, id, status string) error {
	_, err := s.pool.Exec(ctx,
		`UPDATE water_systems SET pump_status = $1, updated_at = NOW() WHERE id = $2`,
		status, id)
	return err
}
