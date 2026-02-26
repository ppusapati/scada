package routes

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	chimw "github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"

	"scada-system/internal/api/handlers"
	"scada-system/internal/api/middleware"
	ws "scada-system/internal/websocket"
)

func NewRouter(
	authHandler *handlers.AuthHandler,
	waterHandler *handlers.WaterHandler,
	solarHandler *handlers.SolarHandler,
	alarmHandler *handlers.AlarmHandler,
	readingHandler *handlers.ReadingHandler,
	authMW *middleware.AuthMiddleware,
	wsHub *ws.Hub,
) http.Handler {
	r := chi.NewRouter()

	// Global middleware
	r.Use(chimw.Logger)
	r.Use(chimw.Recoverer)
	r.Use(chimw.RequestID)
	r.Use(chimw.RealIP)
	r.Use(chimw.Compress(5))
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://localhost:5173", "http://localhost:3000"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type"},
		ExposedHeaders:   []string{"Link"},
		AllowCredentials: true,
		MaxAge:           300,
	}))

	// Health check
	r.Get("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status":"ok","service":"scada-backend"}`))
	})

	// Public routes
	r.Post("/api/v1/auth/login", authHandler.Login)

	// WebSocket endpoint (auth via query param token)
	r.Get("/ws", wsHub.HandleWebSocket)

	// Protected API routes
	r.Group(func(r chi.Router) {
		r.Use(authMW.Authenticate)

		// User management
		r.Get("/api/v1/auth/me", authHandler.Me)
		r.With(authMW.RequireRole("admin")).Post("/api/v1/auth/register", authHandler.Register)

		// ── Water Subsystem ──
		r.Route("/api/v1/water", func(r chi.Router) {
			r.Get("/systems", waterHandler.GetAllSystems)
			r.Get("/systems/{id}", waterHandler.GetSystem)
			r.Get("/devices", waterHandler.GetDevices)

			// Control operations require operator or admin role
			r.Group(func(r chi.Router) {
				r.Use(authMW.RequireRole("admin", "operator"))
				r.Post("/systems/{id}/pump", waterHandler.ControlPump)
				r.Post("/systems/{id}/valve", waterHandler.ControlValve)
				r.Put("/systems/{id}/mode", waterHandler.SetMode)
			})
		})

		// ── Solar Subsystem ──
		r.Route("/api/v1/solar", func(r chi.Router) {
			r.Get("/systems", solarHandler.GetAllSystems)
			r.Get("/systems/{id}", solarHandler.GetSystem)
			r.Get("/systems/{id}/production", solarHandler.GetDailyProduction)
			r.Get("/devices", solarHandler.GetDevices)

			r.Group(func(r chi.Router) {
				r.Use(authMW.RequireRole("admin", "operator"))
				r.Post("/systems/{id}/inverter", solarHandler.ControlInverter)
				r.Put("/systems/{id}/mode", solarHandler.SetMode)
			})
		})

		// ── Alarms ──
		r.Route("/api/v1/alarms", func(r chi.Router) {
			r.Get("/active", alarmHandler.GetActive)
			r.Get("/history", alarmHandler.GetHistory)
			r.Get("/counts", alarmHandler.GetCounts)

			r.Group(func(r chi.Router) {
				r.Use(authMW.RequireRole("admin", "operator"))
				r.Post("/{id}/acknowledge", alarmHandler.Acknowledge)
				r.Post("/{id}/clear", alarmHandler.Clear)
			})
		})

		// ── Sensor Readings / Historical Data ──
		r.Route("/api/v1/readings", func(r chi.Router) {
			r.Get("/device/{deviceId}/latest", readingHandler.GetLatest)
			r.Get("/device/{deviceId}/history", readingHandler.GetHistory)
			r.Get("/device/{deviceId}/aggregated", readingHandler.GetAggregated)
		})
	})

	return r
}
