import { useState } from "react";
import "./Sidebar.css";

interface ChannelProps {
	name?: string;
	addBtn?: boolean;
	active?: boolean;
	hasUnread?: boolean;
	onClick?: () => void;
}

const Channel = ({
	name,
	addBtn = false,
	active = false,
	hasUnread = false,
	onClick,
}: ChannelProps) => {
	if (addBtn) {
		return (
			<div className="sidebar-channel add-btn" onClick={onClick}>
				+
			</div>
		);
	}

	if (!name) return null;

	const displayName = name
		.replace("peerboard/v1/", "")
		.substring(0, 2)
		.toLowerCase();
	return (
		<div
			className={`sidebar-channel ${active ? "active" : ""}`}
			onClick={onClick}
		>
			{displayName}
			{hasUnread && <span className="unread-dot" />}
		</div>
	);
};

interface SubscribeModalProps {
	subscribedTopics: Set<string>;
	onSubscribe: (topic: string) => void;
	onUnsubscribe: (topic: string) => void;
	onClose: () => void;
}

const SubscribeModal = ({
	subscribedTopics,
	onSubscribe,
	onUnsubscribe,
	onClose,
}: SubscribeModalProps) => {
	const [input, setInput] = useState("");

	function handleSubscribe() {
		const name = input.trim();
		const full = `peerboard/v1/${name}`;
		if (!full || subscribedTopics.has(full)) return;
		onSubscribe(full);
		setInput("");
	}

	return (
		<div className="modal-overlay" onClick={onClose}>
			<div className="modal" onClick={(e) => e.stopPropagation()}>
				<div className="modal-header">
					<span>Channels</span>
					<button className="modal-close" onClick={onClose}>
						×
					</button>
				</div>

				<div className="input-area">
					<input
						type="text"
						placeholder="New channel name..."
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={(e) =>
							e.key === "Enter" && handleSubscribe()
						}
						autoFocus
					/>
					<button
						onClick={handleSubscribe}
						disabled={
							!input.trim() || subscribedTopics.has(input.trim())
						}
					>
						Join
					</button>
				</div>

				<div className="modal-channel-list">
					{[...subscribedTopics].map((topic) => (
						<div className="modal-channel-row" key={topic}>
							<span className="modal-channel-name">
								{topic.replace("peerboard/v1/", "")}
							</span>
							<button
								className="modal-unsub-btn"
								onClick={() => onUnsubscribe(topic)}
							>
								Leave
							</button>
						</div>
					))}
					{subscribedTopics.size === 0 && (
						<p className="modal-empty">No channels yet.</p>
					)}
				</div>
			</div>
		</div>
	);
};

interface SidebarProps {
	subscribedTopics: Set<string>;
	onSubscribe: (topic: string) => void;
	onUnsubscribe: (topic: string) => void;
	activeTopic: string | null;
	onTopicSelect: (topic: string) => void;
	unreadTopics: Set<string>;
}

export const Sidebar = ({
	subscribedTopics,
	onSubscribe,
	onUnsubscribe,
	activeTopic,
	onTopicSelect,
	unreadTopics,
}: SidebarProps) => {
	const [modalOpen, setModalOpen] = useState(false);

	function handleSubscribe(topic: string) {
		onSubscribe(topic);
	}

	function handleUnsubscribe(topic: string) {
		onUnsubscribe(topic);
	}

	return (
		<div className="sidebar-base">
			<div className="sidebar-stack">
				{[...subscribedTopics].map((topic) => (
					<Channel
						key={topic}
						name={topic}
						active={topic === activeTopic}
						hasUnread={unreadTopics.has(topic)}
						onClick={() => onTopicSelect(topic)}
					/>
				))}
				<Channel addBtn onClick={() => setModalOpen(true)} />
			</div>

			{modalOpen && (
				<SubscribeModal
					subscribedTopics={subscribedTopics}
					onSubscribe={handleSubscribe}
					onUnsubscribe={handleUnsubscribe}
					onClose={() => setModalOpen(false)}
				/>
			)}
		</div>
	);
};

export const RightSideBar = () => {
	return <div className="sidebar-base right-side"></div>;
};
