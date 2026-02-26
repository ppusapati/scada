package websocket

import (
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		return true // Configure properly in production
	},
}

type Message struct {
	Type      string      `json:"type"`
	Subsystem string      `json:"subsystem,omitempty"`
	DeviceID  string      `json:"device_id,omitempty"`
	Data      interface{} `json:"data"`
	Timestamp time.Time   `json:"timestamp"`
}

type Subscription struct {
	Subsystems []string // empty = all
	DeviceIDs  []string // empty = all
}

type client struct {
	conn         *websocket.Conn
	send         chan []byte
	hub          *Hub
	subscription Subscription
	mu           sync.Mutex
}

type Hub struct {
	clients    map[*client]bool
	broadcast  chan Message
	register   chan *client
	unregister chan *client
	mu         sync.RWMutex
}

func NewHub() *Hub {
	return &Hub{
		clients:    make(map[*client]bool),
		broadcast:  make(chan Message, 256),
		register:   make(chan *client),
		unregister: make(chan *client),
	}
}

func (h *Hub) Run() {
	for {
		select {
		case c := <-h.register:
			h.mu.Lock()
			h.clients[c] = true
			h.mu.Unlock()
			fmt.Printf("[WS] Client connected (%d total)\n", len(h.clients))

		case c := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[c]; ok {
				delete(h.clients, c)
				close(c.send)
			}
			h.mu.Unlock()
			fmt.Printf("[WS] Client disconnected (%d total)\n", len(h.clients))

		case msg := <-h.broadcast:
			data, err := json.Marshal(msg)
			if err != nil {
				continue
			}
			h.mu.RLock()
			for c := range h.clients {
				if c.matchesSubscription(msg) {
					select {
					case c.send <- data:
					default:
						go func(cl *client) {
							h.unregister <- cl
						}(c)
					}
				}
			}
			h.mu.RUnlock()
		}
	}
}

func (h *Hub) Broadcast(msg Message) {
	select {
	case h.broadcast <- msg:
	default:
		fmt.Println("[WS] Broadcast channel full, dropping message")
	}
}

func (h *Hub) ClientCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.clients)
}

func (h *Hub) HandleWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		fmt.Printf("[WS] Upgrade error: %v\n", err)
		return
	}

	c := &client{
		conn: conn,
		send: make(chan []byte, 256),
		hub:  h,
		subscription: Subscription{},
	}

	h.register <- c

	go c.writePump()
	go c.readPump()
}

func (c *client) readPump() {
	defer func() {
		c.hub.unregister <- c
		c.conn.Close()
	}()

	c.conn.SetReadLimit(4096)
	c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, message, err := c.conn.ReadMessage()
		if err != nil {
			break
		}

		// Handle subscription messages from client
		var sub struct {
			Type       string   `json:"type"`
			Subsystems []string `json:"subsystems"`
			DeviceIDs  []string `json:"device_ids"`
		}
		if err := json.Unmarshal(message, &sub); err == nil && sub.Type == "subscribe" {
			c.mu.Lock()
			c.subscription = Subscription{
				Subsystems: sub.Subsystems,
				DeviceIDs:  sub.DeviceIDs,
			}
			c.mu.Unlock()
		}
	}
}

func (c *client) writePump() {
	ticker := time.NewTicker(30 * time.Second)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if !ok {
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}
			if err := c.conn.WriteMessage(websocket.TextMessage, message); err != nil {
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

func (c *client) matchesSubscription(msg Message) bool {
	c.mu.Lock()
	defer c.mu.Unlock()

	sub := c.subscription

	// If no filters, receive everything
	if len(sub.Subsystems) == 0 && len(sub.DeviceIDs) == 0 {
		return true
	}

	// Check subsystem filter
	if len(sub.Subsystems) > 0 && msg.Subsystem != "" {
		found := false
		for _, s := range sub.Subsystems {
			if s == msg.Subsystem {
				found = true
				break
			}
		}
		if !found {
			return false
		}
	}

	// Check device filter
	if len(sub.DeviceIDs) > 0 && msg.DeviceID != "" {
		found := false
		for _, d := range sub.DeviceIDs {
			if d == msg.DeviceID {
				found = true
				break
			}
		}
		if !found {
			return false
		}
	}

	return true
}
