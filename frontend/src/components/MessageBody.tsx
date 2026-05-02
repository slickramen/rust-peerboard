import type { MessageGroup } from "../types/types";

interface MessageBodyProps {
	connected: boolean;
	activeTopic: string | null;
	groupedMessages: MessageGroup[];
}

function formatTime(timestamp: number): string {
	return new Date(timestamp * 1000).toLocaleTimeString([], {
		hour: "numeric",
		minute: "2-digit",
		hour12: true,
	});
}

const MessageBody = ({
	connected,
	activeTopic,
	groupedMessages,
}: MessageBodyProps) => {
	const topicName = activeTopic?.replace("peerboard/v1/", "");

	if (!connected) {
		return (
			<>
				<div className="welcome-to-channel">
					<span className="welcome-message">Not connected</span>
					<span className="welcome-message-subtitle">
						Connect to the server to begin chatting.
					</span>
				</div>
			</>
		);
	}

	if (!activeTopic) {
		return (
			<>
				<div className="welcome-to-channel">
					<span className="welcome-message">No topic selected</span>
					<span className="welcome-message-subtitle">
						Subscribe to a topic to begin chatting.
					</span>
				</div>
			</>
		);
	}

	if (groupedMessages.length === 0) {
		return (
			<>
				<div className="welcome-to-channel">
					<span className="welcome-message">
						Welcome to #{topicName}
					</span>
					<span className="welcome-message-subtitle">
						This is the beginning of the topic.
					</span>
				</div>

				<div className="no-messages">
					<span className="no-messages-label">No messages yet.</span>
				</div>
			</>
		);
	}

	return (
		<>
			<div className="welcome-to-channel">
				<span className="welcome-message">Welcome to #{topicName}</span>
				<span className="welcome-message-subtitle">
					This is the beginning of the topic.
				</span>
			</div>

			{groupedMessages.map((group, i) => (
				<div key={i} className="message-group">
					<div className="avatar chat">
						<img src={`/icons/${group.avatar_id}.png`} />
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
							<div key={msg.message_id} className="message-line">
								<span className="content">{msg.content}</span>
								<span className="time show-on-hover">
									{formatTime(msg.timestamp)}
								</span>
							</div>
						))}
					</div>
				</div>
			))}
		</>
	);
};

export default MessageBody;
