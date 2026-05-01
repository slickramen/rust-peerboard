import getAvatar from "./hashString";
import type { ChatMessage, MessageGroup } from "./types";

export function groupMessages(messages: ChatMessage[]): MessageGroup[] {
	return messages.reduce<MessageGroup[]>((groups, msg) => {
		const lastGroup = groups[groups.length - 1];

		if (lastGroup && lastGroup.peer_id === msg.peer_id) {
			lastGroup.messages.push(msg);
		} else {
			const avatar_number = getAvatar(msg.peer_id);

			groups.push({
				nickname: msg.nickname,
				peer_id: msg.peer_id,
				avatar_id: avatar_number,
				messages: [msg],
				timestamp: msg.timestamp,
			});
		}

		return groups;
	}, []);
}
