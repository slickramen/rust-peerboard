import { useState, useEffect, useRef, useMemo } from "react";
import { useWebSocket } from "./useWebSocket";
import { Sidebar, RightSideBar } from "./components/Sidebar";
import Header from "./components/Header";

import "./App.css";
import { groupMessages } from "./groupMsg";

function formatTime(timestamp: number): string {
	return new Date(timestamp * 1000).toLocaleTimeString();
}

export default function App() {
	const { messages, connected, send } = useWebSocket();
	const [input, setInput] = useState("");
	const bottomRef = useRef<HTMLDivElement>(null);

	const username = "anon";

	useEffect(() => {
		bottomRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		if (!input.trim()) return;
		send(input);
		setInput("");
	}

	const groupedMessages = useMemo(
		() =>
			groupMessages(
				[...messages].sort((a, b) => a.timestamp - b.timestamp),
			),
		[messages],
	);

	return (
		<div className="app">
			<Header connected={connected} username={username} />

			<div className="page-body">
				<Sidebar />

				<div className="channel-body">
					<div className="message-list">
						{groupedMessages.map((group, i) => (
							<div key={i} className="message-group">
								<div className="avatar" />

								<div className="text-content">
									<div className="group-title">
										<span className="meta">
											<strong>{group.nickname}</strong>
										</span>
										<span className="time">
											{formatTime(group.timestamp)}
										</span>
									</div>

									{group.messages.map((msg) => (
										<div
											key={msg.message_id}
											className="message-line"
										>
											<span className="content">
												{msg.content}
											</span>
											<span className="time show-on-hover">
												{formatTime(msg.timestamp)}
											</span>
										</div>
									))}
								</div>
							</div>
						))}
						<div ref={bottomRef} />
					</div>

					<form className="input-area" onSubmit={handleSubmit}>
						<input
							type="text"
							value={input}
							onChange={(e) => setInput(e.target.value)}
							placeholder="Type a message..."
							maxLength={4096}
							disabled={!connected}
						/>
						<button
							type="submit"
							className="send-button"
							disabled={!connected || !input.trim()}
						>
							Send
						</button>
					</form>
				</div>

				<RightSideBar />
			</div>
		</div>
	);
}
