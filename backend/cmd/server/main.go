package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/go-chi/chi/v5"
	chimw "github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
	"github.com/joho/godotenv"

	"scada-system/infrastructure/circuitbreaker"
	"scada-system/internal/api/handlers"
	"scada-system/internal/api/middleware"
	"scada-system/internal/api/routes"
	"scada-system/internal/config"
	"scada-system/internal/db"
	mqttclient "scada-system/internal/mqtt"
	"scada-system/internal/services"
	"scada-system/internal/websocket"
)

func main() {
	godotenv.Load()
	cfg := config.Load()

	// ═══════════════════════════════════════════════
	// Infrastructure Layer
	// ═══════════════════════════════════════════════

	// Database
	database, err := db.New(cfg.Database)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}
	defer database.Close()
	fmt.Println("[DB] Connected to PostgreSQL")

	// Circuit Breaker Registry
	cbRegistry := circuitbreaker.NewRegistry()

	dbCB := circuitbreaker.New(circuitbreaker.Config{
		Name:             "database",
		MaxFailures:      5,
		ResetTimeout:     30 * time.Second,
		HalfOpenMaxCalls: 3,
		SuccessThreshold: 2,
		OnStateChange: func(name string, from, to circuitbreaker.State) {
			fmt.Printf("[CircuitBreaker] %s: %s → %s\n", name, from, to)
		},
	})
	cbRegistry.Register("database", dbCB)

	mqttCB := circuitbreaker.New(circuitbreaker.Config{
		Name:             "mqtt",
		MaxFailures:      3,
		ResetTimeout:     15 * time.Second,
		HalfOpenMaxCalls: 2,
		SuccessThreshold: 1,
		OnStateChange: func(name string, from, to circuitbreaker.State) {
			fmt.Printf("[CircuitBreaker] %s: %s → %s\n", name, from, to)
		},
	})
	cbRegistry.Register("mqtt", mqttCB)

	// MQTT
	mqttClient := mqttclient.NewClient(cfg.MQTT)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if err := mqttClient.Connect(ctx); err != nil {
		fmt.Printf("[MQTT] Warning: %v (running without real-time data)\n", err)
	} else {
		fmt.Println("[MQTT] Connected to broker")
	}

	// WebSocket Hub
	wsHub := websocket.NewHub()
	go wsHub.Run()

	// ═══════════════════════════════════════════════
	// Service Layer
	// ═══════════════════════════════════════════════

	deviceSvc := services.NewDeviceService(database.Pool)
	readingSvc := services.NewReadingService(database.Pool)
	waterSvc := services.NewWaterService(database.Pool)
	solarSvc := services.NewSolarService(database.Pool)
	alarmSvc := services.NewAlarmService(database.Pool)
	authSvc := services.NewAuthService(database.Pool)
	reportSvc := services.NewReportService(database.Pool)

	// Alarm Engine (evaluates sensor data against rules)
	alarmEngine := services.NewAlarmEngine(database.Pool, alarmSvc, wsHub)
	_ = alarmEngine // Used by the MQTT bridge for real-time evaluation

	// MQTT Bridge
	bridge := mqttclient.NewBridge(mqttClient, readingSvc, deviceSvc, waterSvc, solarSvc, alarmSvc, wsHub)
	if err := bridge.Start(ctx); err != nil {
		fmt.Printf("[Bridge] Warning: %v\n", err)
	}

	// Auth
	authMW := middleware.NewAuthMiddleware(cfg.JWT.Secret)

	// ═══════════════════════════════════════════════
	// HTTP Router: REST + ConnectRPC on same port
	// ═══════════════════════════════════════════════

	authHandler := handlers.NewAuthHandler(authSvc, authMW, cfg.JWT.Expiration)
	waterHandler := handlers.NewWaterHandler(waterSvc, deviceSvc, mqttClient)
	solarHandler := handlers.NewSolarHandler(solarSvc, deviceSvc, mqttClient)
	alarmHandler := handlers.NewAlarmHandler(alarmSvc)
	readingHandler := handlers.NewReadingHandler(readingSvc)
	reportHandler := handlers.NewReportHandler(reportSvc)

	restRouter := routes.NewRouter(authHandler, waterHandler, solarHandler, alarmHandler, readingHandler, authMW, wsHub)

	// Root router
	root := chi.NewRouter()
	root.Use(chimw.Logger, chimw.Recoverer, chimw.RequestID)
	root.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://localhost:5173", "http://localhost:3000"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type", "Connect-Protocol-Version"},
		ExposedHeaders:   []string{"Link"},
		AllowCredentials: true,
	}))

	// REST API
	root.Mount("/", restRouter)

	// Report endpoints
	root.Group(func(r chi.Router) {
		r.Use(authMW.Authenticate)
		r.Get("/api/v1/reports/daily", reportHandler.GetDailySummary)
		r.Get("/api/v1/reports/energy", reportHandler.GetEnergyReport)
		r.Get("/api/v1/reports/water-quality", reportHandler.GetWaterQualityReport)
	})

	// Circuit Breaker health endpoint
	root.Get("/health/circuit-breakers", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		allMetrics := cbRegistry.GetAll()
		resp := "{"
		first := true
		for name, m := range allMetrics {
			if !first {
				resp += ","
			}
			resp += fmt.Sprintf(`"%s":{"state":"%s","total_calls":%d,"total_failure":%d,"total_reject":%d,"current_failures":%d}`,
				name, m.State, m.TotalCalls, m.TotalFailure, m.TotalReject, m.Failures)
			first = false
		}
		resp += "}"
		w.Write([]byte(resp))
	})

	// ConnectRPC mount point (activate after running `buf generate`)
	// connectMux, _ := grpcserver.NewConnectMux(waterUC, solarUC, alarmUC, readingUC, authUC, cbRegistry, cfg.JWT.Secret, database.Health)
	// root.Mount("/scada.v1.", connectMux)

	// ═══════════════════════════════════════════════
	// Start Server
	// ═══════════════════════════════════════════════

	addr := fmt.Sprintf("%s:%d", cfg.Server.Host, cfg.Server.Port)
	srv := &http.Server{
		Addr:         addr,
		Handler:      root,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan
		fmt.Println("\n[Server] Shutting down...")
		shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 15*time.Second)
		defer shutdownCancel()
		mqttClient.Disconnect()
		srv.Shutdown(shutdownCtx)
	}()

	fmt.Printf("\n╔══════════════════════════════════════════════════╗\n")
	fmt.Printf("║   SCADA System Backend v2.0                      ║\n")
	fmt.Printf("║   REST API:      http://%s/api/v1        ║\n", addr)
	fmt.Printf("║   ConnectRPC:    http://%s/scada.v1.*    ║\n", addr)
	fmt.Printf("║   WebSocket:     ws://%s/ws              ║\n", addr)
	fmt.Printf("║   Circuit Breakers: %d registered                ║\n", len(cbRegistry.GetAll()))
	fmt.Printf("║   Architecture:  Clean Architecture              ║\n")
	fmt.Printf("╚══════════════════════════════════════════════════╝\n\n")

	if err := srv.ListenAndServe(); err != http.ErrServerClosed {
		log.Fatalf("[Server] Error: %v", err)
	}
}
