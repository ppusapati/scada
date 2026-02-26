package handler

import (
	"context"

	"connectrpc.com/connect"

	scadav1 "scada-system/gen/scada/v1"
	"scada-system/gen/scada/v1/scadav1connect"
	"scada-system/infrastructure/circuitbreaker"
)

type HealthServer struct {
	scadav1connect.UnimplementedHealthServiceHandler
	cbRegistry *circuitbreaker.Registry
	dbCheck    func(ctx context.Context) error
}

func NewHealthServer(cbRegistry *circuitbreaker.Registry, dbCheck func(ctx context.Context) error) *HealthServer {
	return &HealthServer{cbRegistry: cbRegistry, dbCheck: dbCheck}
}

func (s *HealthServer) Check(ctx context.Context, req *connect.Request[scadav1.HealthCheckRequest]) (*connect.Response[scadav1.HealthCheckResponse], error) {
	components := map[string]string{}

	// Check DB
	if err := s.dbCheck(ctx); err != nil {
		components["database"] = "unhealthy: " + err.Error()
	} else {
		components["database"] = "healthy"
	}

	// Check circuit breakers
	for name, metrics := range s.cbRegistry.GetAll() {
		components["cb_"+name] = metrics.State
	}

	return connect.NewResponse(&scadav1.HealthCheckResponse{
		Status:     "ok",
		Version:    "1.0.0",
		Components: components,
	}), nil
}

func (s *HealthServer) GetCircuitBreakers(ctx context.Context, req *connect.Request[scadav1.GetCircuitBreakersRequest]) (*connect.Response[scadav1.GetCircuitBreakersResponse], error) {
	allMetrics := s.cbRegistry.GetAll()
	breakers := make([]*scadav1.CircuitBreakerStatus, 0, len(allMetrics))

	for _, m := range allMetrics {
		breakers = append(breakers, &scadav1.CircuitBreakerStatus{
			Name:         m.Name,
			State:        m.State,
			TotalCalls:   m.TotalCalls,
			TotalSuccess: m.TotalSuccess,
			TotalFailure: m.TotalFailure,
			TotalReject:  m.TotalReject,
		})
	}

	return connect.NewResponse(&scadav1.GetCircuitBreakersResponse{
		Breakers: breakers,
	}), nil
}
