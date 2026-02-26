package water

import (
	"context"
	"fmt"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
	"scada-system/domain/valueobject"
)

// UseCase encapsulates all water subsystem business logic
type UseCase struct {
	waterRepo   repository.WaterRepository
	deviceRepo  repository.DeviceRepository
	mqttPub     repository.MQTTPublisher
	broadcaster repository.RealtimeBroadcaster
}

func NewUseCase(
	waterRepo repository.WaterRepository,
	deviceRepo repository.DeviceRepository,
	mqttPub repository.MQTTPublisher,
	broadcaster repository.RealtimeBroadcaster,
) *UseCase {
	return &UseCase{
		waterRepo:   waterRepo,
		deviceRepo:  deviceRepo,
		mqttPub:     mqttPub,
		broadcaster: broadcaster,
	}
}

func (uc *UseCase) GetAllSystems(ctx context.Context) ([]entity.WaterSystem, error) {
	return uc.waterRepo.FindAll(ctx)
}

func (uc *UseCase) GetSystem(ctx context.Context, id string) (*entity.WaterSystem, error) {
	system, err := uc.waterRepo.FindByID(ctx, id)
	if err != nil {
		return nil, domainerr.NewNotFound("WaterSystem", id)
	}
	return system, nil
}

func (uc *UseCase) ControlPump(ctx context.Context, systemID string, action string, userID string) error {
	if action != "start" && action != "stop" {
		return domainerr.NewValidation("action", "must be 'start' or 'stop'")
	}

	system, err := uc.waterRepo.FindByID(ctx, systemID)
	if err != nil {
		return domainerr.NewNotFound("WaterSystem", systemID)
	}

	// Business rule: can't start pump in maintenance mode
	if action == "start" && system.OperatingMode == entity.ModeMaintenance {
		return domainerr.NewValidation("action", "cannot start pump in maintenance mode")
	}

	// Send command via MQTT
	cmd := valueobject.CommandRequest{
		DeviceID:   systemID,
		Action:     action + "_pump",
		Parameters: map[string]string{"system_id": systemID},
		IssuedBy:   userID,
	}
	if err := uc.mqttPub.PublishCommand(ctx, "water", systemID, cmd); err != nil {
		return fmt.Errorf("publish pump command: %w", err)
	}

	// Optimistic local state update
	status := entity.PumpRunning
	if action == "stop" {
		status = entity.PumpStopped
	}
	return uc.waterRepo.SetPumpStatus(ctx, systemID, status)
}

func (uc *UseCase) ControlValve(ctx context.Context, systemID, valveID string, position float64, userID string) error {
	if position < 0 || position > 100 {
		return domainerr.NewValidation("position", "must be between 0 and 100")
	}

	cmd := valueobject.CommandRequest{
		DeviceID: systemID,
		Action:   "set_valve",
		Parameters: map[string]string{
			"valve_id": valveID,
			"position": fmt.Sprintf("%.1f", position),
		},
		IssuedBy: userID,
	}
	return uc.mqttPub.PublishCommand(ctx, "water", systemID, cmd)
}

func (uc *UseCase) SetOperatingMode(ctx context.Context, systemID string, mode entity.OperatingMode) error {
	if !mode.IsValid() {
		return domainerr.NewValidation("mode", "must be auto, manual, or maintenance")
	}
	return uc.waterRepo.SetOperatingMode(ctx, systemID, mode)
}

func (uc *UseCase) GetDevices(ctx context.Context) ([]entity.Device, error) {
	return uc.deviceRepo.FindAll(ctx, "water")
}

func (uc *UseCase) ProcessTelemetry(ctx context.Context, systemID string, metrics map[string]interface{}) error {
	if err := uc.waterRepo.UpdateFromTelemetry(ctx, systemID, metrics); err != nil {
		return err
	}
	uc.broadcaster.BroadcastTelemetry("water", systemID, metrics)
	return nil
}
