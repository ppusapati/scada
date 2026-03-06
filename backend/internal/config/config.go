package config

import (
	"fmt"
	"os"
	"strconv"
)

type Config struct {
	Server   ServerConfig
	Database DatabaseConfig
	MQTT     MQTTConfig
	DDS      DDSConfig
	JWT      JWTConfig
}

type DDSConfig struct {
	Enabled       bool
	DomainID      int
	MulticastAddr string
	Interface     string // network interface for multicast (e.g. "eth0")
	FallbackMQTT  bool   // fall back to MQTT when DDS is unavailable
}

type ServerConfig struct {
	Host string
	Port int
}

type DatabaseConfig struct {
	Host     string
	Port     int
	User     string
	Password string
	DBName   string
	SSLMode  string
}

type MQTTConfig struct {
	Broker   string
	Port     int
	ClientID string
	Username string
	Password string
}

type JWTConfig struct {
	Secret     string
	Expiration int // hours
}

func Load() (*Config, error) {
	dbPassword := requireEnv("DB_PASSWORD")
	jwtSecret := requireEnv("JWT_SECRET")

	if dbPassword == "" {
		return nil, fmt.Errorf("DB_PASSWORD environment variable is required")
	}
	if jwtSecret == "" {
		return nil, fmt.Errorf("JWT_SECRET environment variable is required")
	}

	return &Config{
		Server: ServerConfig{
			Host: getEnv("SERVER_HOST", "0.0.0.0"),
			Port: getEnvInt("SERVER_PORT", 8080),
		},
		Database: DatabaseConfig{
			Host:     getEnv("DB_HOST", "localhost"),
			Port:     getEnvInt("DB_PORT", 5432),
			User:     getEnv("DB_USER", "scada"),
			Password: dbPassword,
			DBName:   getEnv("DB_NAME", "scada_db"),
			SSLMode:  getEnv("DB_SSLMODE", "require"),
		},
		MQTT: MQTTConfig{
			Broker:   getEnv("MQTT_BROKER", "localhost"),
			Port:     getEnvInt("MQTT_PORT", 1883),
			ClientID: getEnv("MQTT_CLIENT_ID", "scada-backend"),
			Username: getEnv("MQTT_USERNAME", ""),
			Password: getEnv("MQTT_PASSWORD", ""),
		},
		DDS: DDSConfig{
			Enabled:       getEnvBool("DDS_ENABLED", true),
			DomainID:      getEnvInt("DDS_DOMAIN_ID", 0),
			MulticastAddr: getEnv("DDS_MULTICAST_ADDR", "239.255.0.1"),
			Interface:     getEnv("DDS_INTERFACE", ""),
			FallbackMQTT:  getEnvBool("DDS_FALLBACK_MQTT", true),
		},
		JWT: JWTConfig{
			Secret:     jwtSecret,
			Expiration: getEnvInt("JWT_EXPIRATION", 24),
		},
	}, nil
}

func requireEnv(key string) string {
	return os.Getenv(key)
}

func (d *DatabaseConfig) DSN() string {
	return "postgres://" + d.User + ":" + d.Password +
		"@" + d.Host + ":" + strconv.Itoa(d.Port) +
		"/" + d.DBName + "?sslmode=" + d.SSLMode
}

func getEnv(key, fallback string) string {
	if val := os.Getenv(key); val != "" {
		return val
	}
	return fallback
}

func getEnvInt(key string, fallback int) int {
	if val := os.Getenv(key); val != "" {
		if i, err := strconv.Atoi(val); err == nil {
			return i
		}
	}
	return fallback
}

func getEnvBool(key string, fallback bool) bool {
	if val := os.Getenv(key); val != "" {
		switch val {
		case "true", "1", "yes", "on":
			return true
		case "false", "0", "no", "off":
			return false
		}
	}
	return fallback
}
