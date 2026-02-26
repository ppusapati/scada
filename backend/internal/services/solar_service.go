package services

import (
	"context"
	"fmt"

	"github.com/jackc/pgx/v5/pgxpool"
	"scada-system/internal/db/models"
)

type SolarService struct {
	pool *pgxpool.Pool
}

func NewSolarService(pool *pgxpool.Pool) *SolarService {
	return &SolarService{pool: pool}
}

func (s *SolarService) GetStatus(ctx context.Context, id string) (*models.SolarSystem, error) {
	var sol models.SolarSystem
	err := s.pool.QueryRow(ctx,
		`SELECT id, name, total_capacity_kw, current_output_kw, irradiance,
			panel_temperature, inverter_status, grid_voltage, grid_frequency,
			daily_energy_kwh, total_energy_mwh, efficiency, operating_mode, updated_at
		FROM solar_systems WHERE id = $1`, id).
		Scan(&sol.ID, &sol.Name, &sol.TotalCapacityKW, &sol.CurrentOutputKW,
			&sol.Irradiance, &sol.PanelTemperature, &sol.InverterStatus,
			&sol.GridVoltage, &sol.GridFrequency, &sol.DailyEnergyKWH,
			&sol.TotalEnergyMWH, &sol.Efficiency, &sol.OperatingMode, &sol.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &sol, nil
}

func (s *SolarService) GetAll(ctx context.Context) ([]models.SolarSystem, error) {
	rows, err := s.pool.Query(ctx,
		`SELECT id, name, total_capacity_kw, current_output_kw, irradiance,
			panel_temperature, inverter_status, grid_voltage, grid_frequency,
			daily_energy_kwh, total_energy_mwh, efficiency, operating_mode, updated_at
		FROM solar_systems ORDER BY name`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var systems []models.SolarSystem
	for rows.Next() {
		var sol models.SolarSystem
		if err := rows.Scan(&sol.ID, &sol.Name, &sol.TotalCapacityKW, &sol.CurrentOutputKW,
			&sol.Irradiance, &sol.PanelTemperature, &sol.InverterStatus,
			&sol.GridVoltage, &sol.GridFrequency, &sol.DailyEnergyKWH,
			&sol.TotalEnergyMWH, &sol.Efficiency, &sol.OperatingMode, &sol.UpdatedAt); err != nil {
			return nil, err
		}
		systems = append(systems, sol)
	}
	return systems, nil
}

func (s *SolarService) UpdateFromMQTT(ctx context.Context, id string, updates map[string]interface{}) error {
	query := `UPDATE solar_systems SET updated_at = NOW()`
	args := []interface{}{}
	argIdx := 1

	fieldMap := map[string]string{
		"current_output_kw":  "current_output_kw",
		"irradiance":         "irradiance",
		"panel_temperature":  "panel_temperature",
		"inverter_status":    "inverter_status",
		"grid_voltage":       "grid_voltage",
		"grid_frequency":     "grid_frequency",
		"daily_energy_kwh":   "daily_energy_kwh",
		"total_energy_mwh":   "total_energy_mwh",
		"efficiency":         "efficiency",
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

func (s *SolarService) SetOperatingMode(ctx context.Context, id, mode string) error {
	_, err := s.pool.Exec(ctx,
		`UPDATE solar_systems SET operating_mode = $1, updated_at = NOW() WHERE id = $2`,
		mode, id)
	return err
}
