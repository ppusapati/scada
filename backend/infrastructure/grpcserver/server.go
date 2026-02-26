package grpcserver

import (
	"context"
	"fmt"
	"net/http"

	"connectrpc.com/connect"
	"golang.org/x/net/http2"
	"golang.org/x/net/http2/h2c"

	"scada-system/gen/scada/v1/scadav1connect"
	"scada-system/infrastructure/circuitbreaker"
	connecthandler "scada-system/interface/connectrpc/handler"
	alarmuc "scada-system/usecase/alarm"
	authuc "scada-system/usecase/auth"
	readinguc "scada-system/usecase/reading"
	solaruc "scada-system/usecase/solar"
	wateruc "scada-system/usecase/water"
)

// ServerConfig holds all configuration for the gRPC/Connect server
type ServerConfig struct {
	JWTSecret  string
	CBRegistry *circuitbreaker.Registry
	DBCheck    func(ctx context.Context) error
}

// NewConnectMux creates an HTTP mux with all ConnectRPC services registered
func NewConnectMux(
	waterUC *wateruc.UseCase,
	solarUC *solaruc.UseCase,
	alarmUC *alarmuc.UseCase,
	readingUC *readinguc.UseCase,
	authUC *authuc.UseCase,
	cbRegistry *circuitbreaker.Registry,
	jwtSecret string,
	dbCheck func(ctx context.Context) error,
) (http.Handler, error) {
	// Public procedures that don't require auth
	publicProcedures := map[string]bool{
		"/scada.v1.AuthService/Login":       true,
		"/scada.v1.HealthService/Check":     true,
	}

	// Control procedures that require operator/admin
	controlProcedures := map[string]bool{
		"/scada.v1.WaterService/ControlPump":       true,
		"/scada.v1.WaterService/ControlValve":      true,
		"/scada.v1.WaterService/SetMode":           true,
		"/scada.v1.SolarService/ControlInverter":   true,
		"/scada.v1.SolarService/SetMode":           true,
		"/scada.v1.AlarmService/AcknowledgeAlarm":  true,
		"/scada.v1.AlarmService/ClearAlarm":        true,
	}

	// Build interceptor chain
	interceptors := connect.WithInterceptors(
		RecoveryInterceptor(),
		LoggingInterceptor(),
		AuthInterceptor(jwtSecret, publicProcedures),
		RoleInterceptor(controlProcedures),
	)

	mux := http.NewServeMux()

	// Register service handlers
	waterPath, waterHandler := scadav1connect.NewWaterServiceHandler(
		connecthandler.NewWaterServer(waterUC), interceptors,
	)
	mux.Handle(waterPath, waterHandler)

	solarPath, solarHandler := scadav1connect.NewSolarServiceHandler(
		connecthandler.NewSolarServer(solarUC), interceptors,
	)
	mux.Handle(solarPath, solarHandler)

	alarmPath, alarmHandler := scadav1connect.NewAlarmServiceHandler(
		connecthandler.NewAlarmServer(alarmUC), interceptors,
	)
	mux.Handle(alarmPath, alarmHandler)

	healthPath, healthHandler := scadav1connect.NewHealthServiceHandler(
		connecthandler.NewHealthServer(cbRegistry, dbCheck), interceptors,
	)
	mux.Handle(healthPath, healthHandler)

	fmt.Println("[ConnectRPC] Services registered:")
	fmt.Printf("  %s\n", waterPath)
	fmt.Printf("  %s\n", solarPath)
	fmt.Printf("  %s\n", alarmPath)
	fmt.Printf("  %s\n", healthPath)

	// Wrap with h2c for HTTP/2 without TLS (development)
	return h2c.NewHandler(mux, &http2.Server{}), nil
}
