import { useEffect, useRef, useState } from "react";

const WS_URL = `ws://${window.location.host}/ws`;

export interface ChatMessage {
	peer_id: string;
	nickname: string;
	content: string;
	timestamp: number;
	message_id: string;
	topic: string;
}

interface UseWebSocketReturn {
	messages: ChatMessage[];
	connected: boolean;
}

export function useWebSocket(): UseWebSocketReturn {
	const ws = useRef<WebSocket | null>(null);
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [connected, setConnected] = useState(false);

	useEffect(() => {
		function connect() {
			const socket = new WebSocket(WS_URL);
			ws.current = socket;

			socket.onopen = () => setConnected(true);

			socket.onmessage = (event: MessageEvent) => {
				try {
					const msg: ChatMessage = JSON.parse(event.data);
					setMessages((prev) => [...prev, msg]);
				} catch (e) {
					console.error("Failed to parse message:", e);
				}
			};

			socket.onclose = () => {
				setConnected(false);
				setTimeout(connect, 2000);
			};

			socket.onerror = (e: Event) => {
				console.error("WebSocket error:", e);
				socket.close();
			};
		}

		connect();
		return () => ws.current?.close();
	}, []);

	return { messages, connected };
}
