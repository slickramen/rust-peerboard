export interface ChatMessage {
	message_id: string;
	nickname: string;
	peer_id: string;
	timestamp: number;
	content: string;
}

export interface MessageGroup {
	nickname: string;
	peer_id: string;
	messages: ChatMessage[];
	timestamp: number;
}
