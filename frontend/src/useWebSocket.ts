import { useCallback, useEffect, useRef, useState } from "react";

const WS_URL = `ws://localhost:3000/ws`;

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
	send: (content: string) => void;
	onInit?: (data: { user_id: string; username: string }) => void;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function useWebSocket(onInit?: (data: any) => void): UseWebSocketReturn {
	const ws = useRef<WebSocket | null>(null);
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [connected, setConnected] = useState(false);

	const send = useCallback((content: string) => {
		if (ws.current?.readyState === WebSocket.OPEN) {
			ws.current.send(JSON.stringify({ content }));
		} else {
			console.warn("WebSocket not connected");
		}
	}, []);

	useEffect(() => {
		function connect() {
			const socket = new WebSocket(WS_URL);
			ws.current = socket;

			socket.onopen = () => setConnected(true);

			socket.onmessage = (event) => {
				const msg = JSON.parse(event.data);

				console.log(msg.type);

				switch (msg.type) {
					case "Init":
						console.log("Connected as:", msg.username, msg.user_id);
						onInit?.({
							user_id: msg.user_id,
							username: msg.username,
						});
						break;

					default:
						setMessages((prev) => {
							if (
								prev.some(
									(m) => m.message_id === msg.message_id,
								)
							) {
								return prev;
							}
							return [...prev, msg];
						});
						break;
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

	return { messages, connected, send };
}
