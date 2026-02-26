package circuitbreaker

import (
	"context"
	"log"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	"scada-system/domain/repository"
	"scada-system/domain/valueobject"
)

// ProtectedDeviceRepo wraps a DeviceRepository with a circuit breaker
type ProtectedDeviceRepo struct {
	inner repository.DeviceRepository
	cb    *CircuitBreaker
}

func NewProtectedDeviceRepo(inner repository.DeviceRepository, cb *CircuitBreaker) *ProtectedDeviceRepo {
	return &ProtectedDeviceRepo{inner: inner, cb: cb}
}

func (r *ProtectedDeviceRepo) FindAll(ctx context.Context, subsystem string) ([]entity.Device, error) {
	return ExecuteWithResult(r.cb, ctx, func(ctx context.Context) ([]entity.Device, error) {
		return r.inner.FindAll(ctx, subsystem)
	})
}

func (r *ProtectedDeviceRepo) FindByID(ctx context.Context, id string) (*entity.Device, error) {
	return ExecuteWithResult(r.cb, ctx, func(ctx context.Context) (*entity.Device, error) {
		return r.inner.FindByID(ctx, id)
	})
}

func (r *ProtectedDeviceRepo) Create(ctx context.Context, device *entity.Device) error {
	return r.cb.Execute(ctx, func(ctx context.Context) error {
		return r.inner.Create(ctx, device)
	})
}

func (r *ProtectedDeviceRepo) UpdateStatus(ctx context.Context, id string, status entity.DeviceStatus) error {
	return r.cb.Execute(ctx, func(ctx context.Context) error {
		return r.inner.UpdateStatus(ctx, id, status)
	})
}

func (r *ProtectedDeviceRepo) Delete(ctx context.Context, id string) error {
	return r.cb.Execute(ctx, func(ctx context.Context) error {
		return r.inner.Delete(ctx, id)
	})
}

// ProtectedReadingRepo wraps ReadingRepository with circuit breaker
type ProtectedReadingRepo struct {
	inner repository.ReadingRepository
	cb    *CircuitBreaker
}

func NewProtectedReadingRepo(inner repository.ReadingRepository, cb *CircuitBreaker) *ProtectedReadingRepo {
	return &ProtectedReadingRepo{inner: inner, cb: cb}
}

func (r *ProtectedReadingRepo) Insert(ctx context.Context, reading *entity.SensorReading) error {
	return r.cb.Execute(ctx, func(ctx context.Context) error {
		return r.inner.Insert(ctx, reading)
	})
}

func (r *ProtectedReadingRepo) InsertBatch(ctx context.Context, readings []entity.SensorReading) error {
	return r.cb.Execute(ctx, func(ctx context.Context) error {
		return r.inner.InsertBatch(ctx, readings)
	})
}

func (r *ProtectedReadingRepo) FindLatest(ctx context.Context, deviceID, metric string) (*entity.SensorReading, error) {
	return ExecuteWithResult(r.cb, ctx, func(ctx context.Context) (*entity.SensorReading, error) {
		return r.inner.FindLatest(ctx, deviceID, metric)
	})
}

func (r *ProtectedReadingRepo) FindHistory(ctx context.Context, deviceID, metric string, tr valueobject.TimeRange, limit int) ([]entity.SensorReading, error) {
	return ExecuteWithResult(r.cb, ctx, func(ctx context.Context) ([]entity.SensorReading, error) {
		return r.inner.FindHistory(ctx, deviceID, metric, tr, limit)
	})
}

func (r *ProtectedReadingRepo) FindAggregated(ctx context.Context, deviceID, metric string, interval valueobject.AggregationInterval, tr valueobject.TimeRange) ([]valueobject.AggregatedReading, error) {
	return ExecuteWithResult(r.cb, ctx, func(ctx context.Context) ([]valueobject.AggregatedReading, error) {
		return r.inner.FindAggregated(ctx, deviceID, metric, interval, tr)
	})
}

// ProtectedMQTTPublisher wraps MQTTPublisher with circuit breaker
type ProtectedMQTTPublisher struct {
	inner repository.MQTTPublisher
	cb    *CircuitBreaker
}

func NewProtectedMQTTPublisher(inner repository.MQTTPublisher, cb *CircuitBreaker) *ProtectedMQTTPublisher {
	return &ProtectedMQTTPublisher{inner: inner, cb: cb}
}

func (p *ProtectedMQTTPublisher) PublishCommand(ctx context.Context, subsystem, deviceID string, cmd valueobject.CommandRequest) error {
	err := p.cb.Execute(ctx, func(ctx context.Context) error {
		return p.inner.PublishCommand(ctx, subsystem, deviceID, cmd)
	})
	if err == ErrCircuitOpen {
		log.Printf("[CircuitBreaker] MQTT publisher circuit open — command to %s/%s rejected", subsystem, deviceID)
		return domainerr.NewUnavailable("MQTT", err)
	}
	return err
}

func (p *ProtectedMQTTPublisher) PublishStatus(ctx context.Context, subsystem, deviceID, status string) error {
	return p.cb.Execute(ctx, func(ctx context.Context) error {
		return p.inner.PublishStatus(ctx, subsystem, deviceID, status)
	})
}

// Registry holds all circuit breakers for monitoring
type Registry struct {
	breakers map[string]*CircuitBreaker
}

func NewRegistry() *Registry {
	return &Registry{breakers: make(map[string]*CircuitBreaker)}
}

func (r *Registry) Register(name string, cb *CircuitBreaker) {
	r.breakers[name] = cb
}

func (r *Registry) GetAll() map[string]Metrics {
	result := make(map[string]Metrics)
	for name, cb := range r.breakers {
		result[name] = cb.Metrics()
	}
	return result
}

func (r *Registry) Get(name string) (*CircuitBreaker, bool) {
	cb, ok := r.breakers[name]
	return cb, ok
}
