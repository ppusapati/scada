package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/joho/godotenv"

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
	// Load .env file if present
	godotenv.Load()

	cfg := config.Load()

	// ── Database ──
	database, err := db.New(cfg.Database)
	if err != nil {
		fmt.Printf("Failed to connect to database: %v\n", err)
		os.Exit(1)
	}
	defer database.Close()
	fmt.Println("[DB] Connected to PostgreSQL")

	// ── Services ──
	deviceSvc := services.NewDeviceService(database.Pool)
	readingSvc := services.NewReadingService(database.Pool)
	waterSvc := services.NewWaterService(database.Pool)
	solarSvc := services.NewSolarService(database.Pool)
	alarmSvc := services.NewAlarmService(database.Pool)
	authSvc := services.NewAuthService(database.Pool)

	// ── MQTT Client ──
	mqttClient := mqttclient.NewClient(cfg.MQTT)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if err := mqttClient.Connect(ctx); err != nil {
		fmt.Printf("[MQTT] Warning: could not connect to broker: %v\n", err)
		fmt.Println("[MQTT] System will operate without real-time IoT data")
	} else {
		fmt.Println("[MQTT] Connected to broker")
	}

	// ── WebSocket Hub ──
	wsHub := websocket.NewHub()
	go wsHub.Run()

	// ── MQTT-to-DB Bridge ──
	bridge := mqttclient.NewBridge(mqttClient, readingSvc, deviceSvc, waterSvc, solarSvc, alarmSvc, wsHub)
	if err := bridge.Start(ctx); err != nil {
		fmt.Printf("[Bridge] Warning: %v\n", err)
	}

	// ── Auth Middleware ──
	authMW := middleware.NewAuthMiddleware(cfg.JWT.Secret)

	// ── API Handlers ──
	authHandler := handlers.NewAuthHandler(authSvc, authMW, cfg.JWT.Expiration)
	waterHandler := handlers.NewWaterHandler(waterSvc, deviceSvc, mqttClient)
	solarHandler := handlers.NewSolarHandler(solarSvc, deviceSvc, mqttClient)
	alarmHandler := handlers.NewAlarmHandler(alarmSvc)
	readingHandler := handlers.NewReadingHandler(readingSvc)

	// ── Router ──
	router := routes.NewRouter(
		authHandler,
		waterHandler,
		solarHandler,
		alarmHandler,
		readingHandler,
		authMW,
		wsHub,
	)

	// ── HTTP Server ──
	addr := fmt.Sprintf("%s:%d", cfg.Server.Host, cfg.Server.Port)
	srv := &http.Server{
		Addr:         addr,
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Graceful shutdown
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

	fmt.Printf("\n╔══════════════════════════════════════════╗\n")
	fmt.Printf("║   SCADA System Backend                   ║\n")
	fmt.Printf("║   Listening on %s              ║\n", addr)
	fmt.Printf("║   API:  http://%s/api/v1        ║\n", addr)
	fmt.Printf("║   WS:   ws://%s/ws             ║\n", addr)
	fmt.Printf("╚══════════════════════════════════════════╝\n\n")

	if err := srv.ListenAndServe(); err != http.ErrServerClosed {
		fmt.Printf("[Server] Error: %v\n", err)
		os.Exit(1)
	}
}
