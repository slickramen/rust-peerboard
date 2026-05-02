import { useState, useEffect, useRef, useMemo } from "react";
import { useWebSocket } from "./useWebSocket";
import { Sidebar, ChannelSelector } from "./components/Sidebar";
import Header from "./components/Header";

import "./App.css";
import { groupMessages } from "./groupMsg";
import getAvatar from "./hashString";
import MessageBody from "./components/MessageBody";

export default function App() {
	const [input, setInput] = useState("");
	const [activeTopic, setActiveTopic] = useState<string | null>(null);
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
		bottomRef.current?.scrollIntoView({ behavior: "instant" });
	}, [messages, activeTopic]);

	function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		if (!input.trim() || !activeTopic) return;

		send(input, activeTopic);
		setInput("");
	}

	function handleUnsubscribe(topic: string) {
		unsubscribe(topic);
		if (topic === activeTopic) {
			setActiveTopic(null);
		}
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

	const topicName = activeTopic?.replace("peerboard/v1/", "");

	return (
		<div className="app">
			<Header
				connected={connected}
				username={username}
				avatar_id={avatar}
			/>

			<div className="page-body">
				<Sidebar />
				<div className="channel-wrapper">
					<ChannelSelector
						connected={connected}
						subscribedTopics={subscribedTopics}
						onSubscribe={subscribe}
						onUnsubscribe={handleUnsubscribe}
						activeTopic={activeTopic}
						onTopicSelect={handleTopicSelect}
						unreadTopics={unreadTopics}
					/>
					<div className="channel-body">
						<div className="message-list">
							<MessageBody
								connected={connected}
								activeTopic={activeTopic}
								groupedMessages={groupedMessages}
							/>

							<div ref={bottomRef} />
						</div>

						<form className="input-area" onSubmit={handleSubmit}>
							<input
								type="text"
								value={input}
								onChange={(e) => setInput(e.target.value)}
								placeholder={
									connected
										? activeTopic
											? `Message #${topicName}...`
											: "Select a topic..."
										: "Connect to the server..."
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
				</div>

				<Sidebar />
			</div>
		</div>
	);
}
