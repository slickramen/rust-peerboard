import { useState } from "react";
import "./Sidebar.css";

interface ChannelProps {
	name?: string;
	addBtn?: boolean;
	active?: boolean;
	hasUnread?: boolean;
	onClick?: () => void;
	index?: number;
}

const Channel = ({
	name,
	addBtn = false,
	active = false,
	hasUnread = false,
	onClick,
	index = 0,
}: ChannelProps) => {
	if (addBtn) {
		return (
			<div className="channel-name add-btn" onClick={onClick}>
				+
			</div>
		);
	}

	if (!name) return null;

	const displayName = name.replace("peerboard/v1/", "");

	return (
		<div
			className={`channel-name ${active ? "active" : ""}`}
			onClick={onClick}
			style={{ zIndex: index }}
		>
			{hasUnread && <span className="unread-dot" />}
			{displayName}
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

	const TOPIC_REGEX = /^[a-z0-9-]+$/;

	const trimmed = input.trim();
	const isValidFormat = trimmed === "" || TOPIC_REGEX.test(trimmed);
	const isAlreadySubscribed = subscribedTopics.has(`peerboard/v1/${trimmed}`);
	const canSubmit = trimmed !== "" && isValidFormat && !isAlreadySubscribed;

	function handleSubscribe() {
		if (!canSubmit) return;
		onSubscribe(`peerboard/v1/${trimmed}`);
		setInput("");
	}

	return (
		<div className="modal-overlay" onClick={onClose}>
			<div className="modal" onClick={(e) => e.stopPropagation()}>
				<div className="modal-header">
					<span>Topics</span>
					<button className="modal-close" onClick={onClose}>
						×
					</button>
				</div>

				<div className="input-area">
					<input
						type="text"
						placeholder="e.g. off-topic"
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={(e) =>
							e.key === "Enter" && handleSubscribe()
						}
						autoFocus
					/>
					<button
						className="join-button"
						onClick={handleSubscribe}
						disabled={!canSubmit}
					>
						Join
					</button>
				</div>

				{trimmed && !isValidFormat && (
					<span className="modal-input-error">
						Only lowercase letters, numbers, and hyphens allowed.
					</span>
				)}
				{trimmed && isAlreadySubscribed && (
					<span className="modal-input-error">
						Already subscribed to this topic.
					</span>
				)}

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
						<p className="modal-empty">No topics yet.</p>
					)}
				</div>
			</div>
		</div>
	);
};

interface ChannelSelectorProps {
	subscribedTopics: Set<string>;
	onSubscribe: (topic: string) => void;
	onUnsubscribe: (topic: string) => void;
	activeTopic: string | null;
	onTopicSelect: (topic: string) => void;
	unreadTopics: Set<string>;
}

export const ChannelSelector = ({
	subscribedTopics,
	onSubscribe,
	onUnsubscribe,
	activeTopic,
	onTopicSelect,
	unreadTopics,
}: ChannelSelectorProps) => {
	const [modalOpen, setModalOpen] = useState(false);

	function handleSubscribe(topic: string) {
		onSubscribe(topic);
	}

	function handleUnsubscribe(topic: string) {
		onUnsubscribe(topic);
	}

	return (
		<div className="channel-header">
			<div className="channel-header-tabs">
				{[...subscribedTopics].map((topic, i) => (
					<Channel
						key={topic}
						name={topic}
						active={topic === activeTopic}
						hasUnread={unreadTopics.has(topic)}
						onClick={() => onTopicSelect(topic)}
						index={subscribedTopics.size - i}
					/>
				))}
			</div>
			<Channel addBtn onClick={() => setModalOpen(true)} />

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

export const Sidebar = () => {
	return <div className="sidebar-base"></div>;
};
