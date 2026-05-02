import { useState, useEffect, useRef, useMemo } from "react";
import { useWebSocket } from "./useWebSocket";
import { Sidebar, RightSideBar } from "./components/Sidebar";
import Header from "./components/Header";

import "./App.css";
import { groupMessages } from "./groupMsg";
import getAvatar from "./hashString";

function formatTime(timestamp: number): string {
	return new Date(timestamp * 1000).toLocaleTimeString();
}

export default function App() {
	const [input, setInput] = useState("");
	const [activeTopic, setActiveTopic] = useState<string | null>("general");
	const [seenCounts, setSeenCounts] = useState<Record<string, number>>({});

	const {
		messages,
		connected,
		localUser,
		send,
		subscribe,
		unsubscribe,
		subscribedTopics,
	} = useWebSocket();

	const username = localUser?.username ?? "anon";
	const avatar = localUser ? getAvatar(localUser.user_id) : 1;

	const bottomRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		bottomRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		if (!input.trim() || !activeTopic) return;

		send(input, activeTopic);
		setInput("");
	}

	const groupedMessages = useMemo(
		() =>
			groupMessages(
				[...messages]
					.filter((m) => m.topic === activeTopic)
					.sort((a, b) => a.timestamp - b.timestamp),
			),
		[messages, activeTopic],
	);

	const unreadTopics = useMemo(() => {
		const unread = new Set<string>();
		for (const topic of subscribedTopics) {
			const count = messages.filter((m) => m.topic === topic).length;
			const seen = seenCounts[topic] ?? 0;
			if (topic !== activeTopic && count > seen) {
				unread.add(topic);
			}
		}
		return unread;
	}, [messages, seenCounts, activeTopic, subscribedTopics]);

	function handleTopicSelect(topic: string) {
		setActiveTopic(topic);
		setSeenCounts((prev) => ({
			...prev,
			[topic]: messages.filter((m) => m.topic === topic).length,
			...(activeTopic
				? {
						[activeTopic]: messages.filter(
							(m) => m.topic === activeTopic,
						).length,
					}
				: {}),
		}));
	}

	if (!connected) {
		return (
			<div className="app">
				<div className="page-body">not connected!</div>
			</div>
		);
	}

	return (
		<div className="app">
			<Header
				connected={connected}
				username={username}
				avatar_id={avatar}
			/>

			<div className="page-body">
				<Sidebar
					subscribedTopics={subscribedTopics}
					onSubscribe={subscribe}
					onUnsubscribe={unsubscribe}
					activeTopic={activeTopic}
					onTopicSelect={handleTopicSelect}
					unreadTopics={unreadTopics}
				/>

				<div className="channel-body">
					<div className="message-list">
						{groupedMessages.map((group, i) => (
							<div key={i} className="message-group">
								<div className="avatar">
									<img
										src={`/icons/${group.avatar_id}.png`}
									/>
								</div>

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
							placeholder={
								activeTopic
									? `Message #${activeTopic}...`
									: "Select a channel..."
							}
							maxLength={4096}
							disabled={!connected || !activeTopic}
						/>
						<button
							type="submit"
							className="send-button"
							disabled={
								!connected || !input.trim() || !activeTopic
							}
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
