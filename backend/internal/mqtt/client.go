package mqtt

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"
	"scada-system/internal/config"
)

// SCADA MQTT Topic Structure:
// scada/water/{device_id}/telemetry    - Sensor readings from water devices
// scada/water/{device_id}/status       - Device status updates
// scada/water/{device_id}/command      - Commands to water devices
// scada/water/{device_id}/response     - Command responses
// scada/solar/{device_id}/telemetry    - Sensor readings from solar devices
// scada/solar/{device_id}/status       - Device status updates
// scada/solar/{device_id}/command      - Commands to solar devices
// scada/solar/{device_id}/response     - Command responses
// scada/alarms/{subsystem}             - Alarm notifications
// scada/system/heartbeat               - System heartbeat

type TelemetryData struct {
	DeviceID  string             `json:"device_id"`
	Timestamp time.Time          `json:"timestamp"`
	Metrics   map[string]float64 `json:"metrics"`
	Quality   int                `json:"quality"`
}

type StatusData struct {
	DeviceID  string    `json:"device_id"`
	Status    string    `json:"status"`
	Timestamp time.Time `json:"timestamp"`
}

type CommandData struct {
	CommandID  string            `json:"command_id"`
	Action     string            `json:"action"`
	Parameters map[string]string `json:"parameters"`
	IssuedBy   string            `json:"issued_by"`
	IssuedAt   time.Time         `json:"issued_at"`
}

type MessageHandler func(topic string, payload []byte)

type Client struct {
	client   pahomqtt.Client
	cfg      config.MQTTConfig
	handlers map[string][]MessageHandler
	mu       sync.RWMutex
}

func NewClient(cfg config.MQTTConfig) *Client {
	return &Client{
		cfg:      cfg,
		handlers: make(map[string][]MessageHandler),
	}
}

func (c *Client) Connect(ctx context.Context) error {
	opts := pahomqtt.NewClientOptions().
		AddBroker(fmt.Sprintf("tcp://%s:%d", c.cfg.Broker, c.cfg.Port)).
		SetClientID(c.cfg.ClientID).
		SetAutoReconnect(true).
		SetConnectRetry(true).
		SetConnectRetryInterval(5 * time.Second).
		SetKeepAlive(30 * time.Second).
		SetCleanSession(false).
		SetOrderMatters(false).
		SetOnConnectHandler(c.onConnect).
		SetConnectionLostHandler(c.onConnectionLost)

	if c.cfg.Username != "" {
		opts.SetUsername(c.cfg.Username)
		opts.SetPassword(c.cfg.Password)
	}

	// Last Will and Testament
	willPayload, _ := json.Marshal(map[string]string{
		"status":    "offline",
		"client_id": c.cfg.ClientID,
	})
	opts.SetWill("scada/system/status", string(willPayload), 1, true)

	c.client = pahomqtt.NewClient(opts)
	token := c.client.Connect()
	if !token.WaitTimeout(10 * time.Second) {
		return fmt.Errorf("mqtt connect timeout")
	}
	return token.Error()
}

func (c *Client) onConnect(client pahomqtt.Client) {
	fmt.Println("[MQTT] Connected to broker")

	// Resubscribe to all registered topics
	c.mu.RLock()
	defer c.mu.RUnlock()
	for topic := range c.handlers {
		client.Subscribe(topic, 1, c.messageHandler)
	}
}

func (c *Client) onConnectionLost(_ pahomqtt.Client, err error) {
	fmt.Printf("[MQTT] Connection lost: %v\n", err)
}

func (c *Client) messageHandler(_ pahomqtt.Client, msg pahomqtt.Message) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	topic := msg.Topic()
	for pattern, handlers := range c.handlers {
		if topicMatches(pattern, topic) {
			for _, h := range handlers {
				go h(topic, msg.Payload())
			}
		}
	}
}

func (c *Client) Subscribe(topic string, handler MessageHandler) error {
	c.mu.Lock()
	c.handlers[topic] = append(c.handlers[topic], handler)
	c.mu.Unlock()

	if c.client != nil && c.client.IsConnected() {
		token := c.client.Subscribe(topic, 1, c.messageHandler)
		token.Wait()
		return token.Error()
	}
	return nil
}

func (c *Client) Publish(topic string, payload interface{}) error {
	data, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshal payload: %w", err)
	}

	token := c.client.Publish(topic, 1, false, data)
	token.Wait()
	return token.Error()
}

func (c *Client) PublishCommand(subsystem, deviceID string, cmd CommandData) error {
	topic := fmt.Sprintf("scada/%s/%s/command", subsystem, deviceID)
	return c.Publish(topic, cmd)
}

func (c *Client) Disconnect() {
	if c.client != nil && c.client.IsConnected() {
		c.client.Disconnect(1000)
	}
}

func (c *Client) IsConnected() bool {
	return c.client != nil && c.client.IsConnected()
}

// topicMatches checks if a topic matches an MQTT wildcard pattern
func topicMatches(pattern, topic string) bool {
	if pattern == topic {
		return true
	}
	if pattern == "#" {
		return true
	}
	// Simple wildcard matching for + and #
	pParts := splitTopic(pattern)
	tParts := splitTopic(topic)

	for i, pp := range pParts {
		if pp == "#" {
			return true
		}
		if i >= len(tParts) {
			return false
		}
		if pp != "+" && pp != tParts[i] {
			return false
		}
	}
	return len(pParts) == len(tParts)
}

func splitTopic(topic string) []string {
	return strings.Split(topic, "/")
}
