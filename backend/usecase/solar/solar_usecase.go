package solar

import (
	"context"
	"fmt"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
	"scada-system/domain/valueobject"
)

type UseCase struct {
	solarRepo   repository.SolarRepository
	deviceRepo  repository.DeviceRepository
	mqttPub     repository.MQTTPublisher
	broadcaster repository.RealtimeBroadcaster
}

func NewUseCase(
	solarRepo repository.SolarRepository,
	deviceRepo repository.DeviceRepository,
	mqttPub repository.MQTTPublisher,
	broadcaster repository.RealtimeBroadcaster,
) *UseCase {
	return &UseCase{
		solarRepo:   solarRepo,
		deviceRepo:  deviceRepo,
		mqttPub:     mqttPub,
		broadcaster: broadcaster,
	}
}

func (uc *UseCase) GetAllSystems(ctx context.Context) ([]entity.SolarSystem, error) {
	return uc.solarRepo.FindAll(ctx)
}

func (uc *UseCase) GetSystem(ctx context.Context, id string) (*entity.SolarSystem, error) {
	system, err := uc.solarRepo.FindByID(ctx, id)
	if err != nil {
		return nil, domainerr.NewNotFound("SolarSystem", id)
	}
	return system, nil
}

func (uc *UseCase) ControlInverter(ctx context.Context, systemID, action, userID string) error {
	validActions := map[string]bool{"enable": true, "disable": true, "reset": true}
	if !validActions[action] {
		return domainerr.NewValidation("action", "must be enable, disable, or reset")
	}

	cmd := valueobject.CommandRequest{
		DeviceID:   systemID,
		Action:     action + "_inverter",
		Parameters: map[string]string{"system_id": systemID},
		IssuedBy:   userID,
	}
	return uc.mqttPub.PublishCommand(ctx, "solar", systemID, cmd)
}

func (uc *UseCase) SetOperatingMode(ctx context.Context, systemID string, mode entity.OperatingMode) error {
	if !mode.IsValid() {
		return domainerr.NewValidation("mode", "must be auto, manual, or maintenance")
	}
	return uc.solarRepo.SetOperatingMode(ctx, systemID, mode)
}

func (uc *UseCase) GetDevices(ctx context.Context) ([]entity.Device, error) {
	return uc.deviceRepo.FindAll(ctx, "solar")
}

func (uc *UseCase) GetDailyProduction(ctx context.Context, systemID string) (map[string]interface{}, error) {
	system, err := uc.solarRepo.FindByID(ctx, systemID)
	if err != nil {
		return nil, domainerr.NewNotFound("SolarSystem", systemID)
	}
	return map[string]interface{}{
		"system_id":         system.ID,
		"daily_energy_kwh":  system.DailyEnergyKWH,
		"total_energy_mwh":  system.TotalEnergyMWH,
		"current_output_kw": system.CurrentOutputKW,
		"efficiency":        system.Efficiency,
	}, nil
}

func (uc *UseCase) ProcessTelemetry(ctx context.Context, systemID string, metrics map[string]interface{}) error {
	if err := uc.solarRepo.UpdateFromTelemetry(ctx, systemID, metrics); err != nil {
		return fmt.Errorf("update solar telemetry: %w", err)
	}
	uc.broadcaster.BroadcastTelemetry("solar", systemID, metrics)
	return nil
}
