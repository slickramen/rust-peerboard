import { useState, useEffect, useRef } from "react";
import { useWebSocket, type ChatMessage } from "./useWebSocket";

function formatTime(timestamp: number): string {
	return new Date(timestamp * 1000).toLocaleTimeString();
}

export default function App() {
	const { messages, connected } = useWebSocket();
	const [input, setInput] = useState("");
	const bottomRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		bottomRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		// sending not yet wired up on the Rust side
	}

	return (
		<div className="app">
			<header>
				<h1>PeerBoard</h1>
				<span
					className={
						connected ? "status connected" : "status disconnected"
					}
				>
					{connected ? "connected" : "disconnected"}
				</span>
			</header>

			<ul className="message-list">
				{messages.map((msg: ChatMessage) => (
					<li key={msg.message_id} className="message">
						<span className="meta">
							<strong>{msg.nickname}</strong>
							<span className="time">
								{formatTime(msg.timestamp)}
							</span>
						</span>
						<span className="content">{msg.content}</span>
					</li>
				))}
				<div ref={bottomRef} />
			</ul>

			<form className="input-area" onSubmit={handleSubmit}>
				<input
					type="text"
					value={input}
					onChange={(e) => setInput(e.target.value)}
					placeholder="Type a message..."
					maxLength={4096}
					disabled={!connected}
				/>
				<button type="submit" disabled={!connected || !input.trim()}>
					Send
				</button>
			</form>
		</div>
	);
}
