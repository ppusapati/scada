package reading

import (
	"context"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
	"scada-system/domain/valueobject"
)

type UseCase struct {
	readingRepo repository.ReadingRepository
}

func NewUseCase(readingRepo repository.ReadingRepository) *UseCase {
	return &UseCase{readingRepo: readingRepo}
}

func (uc *UseCase) GetLatest(ctx context.Context, deviceID, metric string) (*entity.SensorReading, error) {
	if metric == "" {
		return nil, domainerr.NewValidation("metric", "required")
	}
	reading, err := uc.readingRepo.FindLatest(ctx, deviceID, metric)
	if err != nil {
		return nil, domainerr.NewNotFound("SensorReading", deviceID+"/"+metric)
	}
	return reading, nil
}

func (uc *UseCase) GetHistory(ctx context.Context, deviceID, metric string, tr valueobject.TimeRange, limit int) ([]entity.SensorReading, error) {
	if metric == "" {
		return nil, domainerr.NewValidation("metric", "required")
	}
	return uc.readingRepo.FindHistory(ctx, deviceID, metric, tr, limit)
}

func (uc *UseCase) GetAggregated(ctx context.Context, deviceID, metric string, interval valueobject.AggregationInterval, tr valueobject.TimeRange) ([]valueobject.AggregatedReading, error) {
	if metric == "" {
		return nil, domainerr.NewValidation("metric", "required")
	}
	if !interval.IsValid() {
		return nil, domainerr.NewValidation("interval", "invalid aggregation interval")
	}
	return uc.readingRepo.FindAggregated(ctx, deviceID, metric, interval, tr)
}

func (uc *UseCase) RecordReading(ctx context.Context, reading *entity.SensorReading) error {
	return uc.readingRepo.Insert(ctx, reading)
}

func (uc *UseCase) RecordBatch(ctx context.Context, readings []entity.SensorReading) error {
	return uc.readingRepo.InsertBatch(ctx, readings)
}
