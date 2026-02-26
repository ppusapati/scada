# SCADA System - Water & Solar Management

A full-stack SCADA (Supervisory Control and Data Acquisition) system for monitoring and controlling Water Treatment and Solar Energy infrastructure.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    SCADA System Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐   │
│  │   Svelte      │◄──►│   Go Backend │◄──►│   PostgreSQL     │   │
│  │   Frontend    │WS  │   (REST+WS)  │    │   + TimescaleDB  │   │
│  │   Dashboard   │    │              │    │                  │   │
│  └──────────────┘    └──────┬───────┘    └──────────────────┘   │
│                             │                                    │
│                        MQTT Broker                               │
│                        (Mosquitto)                               │
│                             │                                    │
│              ┌──────────────┼──────────────┐                    │
│              │              │              │                     │
│  ┌───────────▼──┐ ┌────────▼───┐ ┌───────▼────────┐           │
│  │ Water Sensors │ │Solar Panels│ │ Flow Meters    │           │
│  │ (Rust FW)    │ │(Rust FW)   │ │ (Rust FW)      │           │
│  └──────────────┘ └────────────┘ └────────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Layer      | Technology        | Purpose                          |
|------------|-------------------|----------------------------------|
| Frontend   | SvelteKit 2       | Real-time SCADA HMI Dashboard    |
| Backend    | Go 1.22+          | REST API, WebSocket, MQTT Bridge |
| Database   | PostgreSQL + TimescaleDB | Time-series + relational data |
| IoT Comm   | MQTT (Mosquitto)  | Device communication protocol    |
| Embedded   | Rust              | Sensor/actuator firmware         |
| Deploy     | Docker + Docker Compose | Container orchestration     |

## Phases

- **Phase 1**: Backend API, Database, MQTT integration
- **Phase 2**: Svelte frontend dashboard with real-time data
- **Phase 3**: Rust embedded firmware for sensors
- **Phase 4**: Alarms, historical data, reporting
- **Phase 5**: Advanced HMI, PLC integration, deployment

## Quick Start

```bash
# Start infrastructure
docker-compose up -d

# Run backend
cd backend && go run cmd/server/main.go

# Run frontend
cd frontend && npm run dev
```
