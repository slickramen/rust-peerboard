export interface ChatMessage {
	message_id: string;
	nickname: string;
	timestamp: number;
	content: string;
}

export interface MessageGroup {
	nickname: string;
	messages: ChatMessage[];
	timestamp: number;
}
