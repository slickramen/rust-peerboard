import { useCallback, useEffect, useRef, useState } from "react";

const WS_URL = `ws://localhost:3000/ws`;

type WsMessageType =
	| "init"
	| "chat"
	| "subscribe"
	| "unsubscribe"
	| "subscribed"
	| "unsubscribed"
	| "error";

interface BaseMessage {
	type: WsMessageType;
}

interface InitMessage extends BaseMessage {
	type: "init";
	user_id: string;
	username: string;
}

interface ErrorMessage extends BaseMessage {
	type: "error";
	message: string;
}

interface SubscribedMessage extends BaseMessage {
	type: "subscribed";
	topic: string;
}

interface UnsubscribedMessage extends BaseMessage {
	type: "unsubscribed";
	topic: string;
}

export interface ChatMessage extends BaseMessage {
	type: "chat";
	peer_id: string;
	nickname: string;
	content: string;
	timestamp: number;
	message_id: string;
	topic: string;
}

type ServerMessage =
	| InitMessage
	| ChatMessage
	| SubscribedMessage
	| UnsubscribedMessage
	| ErrorMessage;

export interface LocalUser {
	user_id: string;
	username: string;
}

interface UseWebSocketReturn {
	messages: ChatMessage[];
	connected: boolean;
	localUser: LocalUser | null;
	send: (content: string, topic?: string) => void;
	subscribe: (topic: string) => void;
	unsubscribe: (topic: string) => void;
	subscribedTopics: Set<string>;
}

export function useWebSocket(): UseWebSocketReturn {
	const ws = useRef<WebSocket | null>(null);
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [connected, setConnected] = useState(false);
	const [localUser, setLocalUser] = useState<LocalUser | null>(null);
	const [subscribedTopics, setSubscribedTopics] = useState<Set<string>>(
		new Set(),
	);

	const sendRaw = useCallback((data: object) => {
		if (ws.current?.readyState === WebSocket.OPEN) {
			ws.current.send(JSON.stringify(data));
		} else {
			console.warn("WebSocket not connected");
		}
	}, []);

	const send = useCallback(
		(content: string, topic = "general") => {
			sendRaw({ type: "chat", topic, content });
		},
		[sendRaw],
	);

	const subscribe = useCallback(
		(topic: string) => {
			sendRaw({ type: "subscribe", topic });
		},
		[sendRaw],
	);

	const unsubscribe = useCallback(
		(topic: string) => {
			sendRaw({ type: "unsubscribe", topic });
		},
		[sendRaw],
	);

	useEffect(() => {
		console.log(messages);
	}, [messages]);

	useEffect(() => {
		function connect() {
			const socket = new WebSocket(WS_URL);
			ws.current = socket;

			socket.onopen = () => setConnected(true);

			socket.onmessage = (event) => {
				const msg = JSON.parse(event.data) as ServerMessage;

				console.log(msg);

				switch (msg.type) {
					case "init":
						setLocalUser({
							user_id: msg.user_id,
							username: msg.username,
						});
						break;

					case "subscribed":
						setSubscribedTopics(
							(prev) => new Set([...prev, msg.topic]),
						);
						break;

					case "unsubscribed":
						setSubscribedTopics((prev) => {
							const next = new Set(prev);
							next.delete(msg.topic);
							return next;
						});
						break;

					case "error":
						console.error("Server error:", msg.message);
						break;

					case "chat":
					default:
						setMessages((prev) => {
							if (
								prev.some(
									(m) => m.message_id === msg.message_id,
								)
							)
								return prev;
							return [...prev, msg];
						});
						break;
				}
			};

			socket.onclose = () => {
				setConnected(false);
				setTimeout(connect, 2000);
			};

			socket.onerror = (e) => {
				console.error("WebSocket error:", e);
				socket.close();
			};
		}

		connect();
		return () => ws.current?.close();
	}, []);

	return {
		messages,
		connected,
		localUser,
		send,
		subscribe,
		unsubscribe,
		subscribedTopics,
	};
}
