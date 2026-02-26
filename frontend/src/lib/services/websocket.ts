import { writable } from 'svelte/store';
import type { WSMessage } from '$lib/types/scada';

export const wsConnected = writable(false);
export const lastMessage = writable<WSMessage | null>(null);

type MessageCallback = (msg: WSMessage) => void;

class ScadaWebSocket {
	private ws: WebSocket | null = null;
	private url: string;
	private callbacks: Map<string, MessageCallback[]> = new Map();
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 20;

	constructor(url: string) {
		this.url = url;
	}

	connect() {
		if (this.ws?.readyState === WebSocket.OPEN) return;

		try {
			this.ws = new WebSocket(this.url);

			this.ws.onopen = () => {
				console.log('[WS] Connected');
				wsConnected.set(true);
				this.reconnectAttempts = 0;
			};

			this.ws.onmessage = (event) => {
				try {
					const msg: WSMessage = JSON.parse(event.data);
					lastMessage.set(msg);

					// Dispatch to type-specific callbacks
					const typeCallbacks = this.callbacks.get(msg.type) || [];
					typeCallbacks.forEach(cb => cb(msg));

					// Dispatch to wildcard callbacks
					const wildcardCallbacks = this.callbacks.get('*') || [];
					wildcardCallbacks.forEach(cb => cb(msg));
				} catch (e) {
					console.error('[WS] Failed to parse message:', e);
				}
			};

			this.ws.onclose = () => {
				console.log('[WS] Disconnected');
				wsConnected.set(false);
				this.scheduleReconnect();
			};

			this.ws.onerror = (error) => {
				console.error('[WS] Error:', error);
			};
		} catch (e) {
			console.error('[WS] Connection failed:', e);
			this.scheduleReconnect();
		}
	}

	private scheduleReconnect() {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			console.error('[WS] Max reconnection attempts reached');
			return;
		}

		const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
		this.reconnectAttempts++;

		console.log(`[WS] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
		this.reconnectTimer = setTimeout(() => this.connect(), delay);
	}

	subscribe(type: string, callback: MessageCallback): () => void {
		if (!this.callbacks.has(type)) {
			this.callbacks.set(type, []);
		}
		this.callbacks.get(type)!.push(callback);

		// Return unsubscribe function
		return () => {
			const cbs = this.callbacks.get(type);
			if (cbs) {
				const idx = cbs.indexOf(callback);
				if (idx >= 0) cbs.splice(idx, 1);
			}
		};
	}

	// Tell the server what data we want
	setSubscription(subsystems: string[], deviceIds: string[] = []) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify({
				type: 'subscribe',
				subsystems,
				device_ids: deviceIds
			}));
		}
	}

	disconnect() {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
		}
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		wsConnected.set(false);
	}
}

// Singleton instance
let instance: ScadaWebSocket | null = null;

export function getWebSocket(): ScadaWebSocket {
	if (!instance) {
		const wsUrl = typeof window !== 'undefined'
			? `ws://${window.location.host}/ws`
			: 'ws://localhost:8080/ws';
		instance = new ScadaWebSocket(wsUrl);
	}
	return instance;
}
