import type { ChatMessage, MessageGroup } from "./types";

export function groupMessages(messages: ChatMessage[]): MessageGroup[] {
	return messages.reduce<MessageGroup[]>((groups, msg) => {
		const lastGroup = groups[groups.length - 1];

		if (lastGroup && lastGroup.nickname === msg.nickname) {
			lastGroup.messages.push(msg);
		} else {
			groups.push({
				nickname: msg.nickname,
				messages: [msg],
				timestamp: msg.timestamp,
			});
		}

		return groups;
	}, []);
}
