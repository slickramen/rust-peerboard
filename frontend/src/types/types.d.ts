export interface MessageGroup {
	nickname: string;
	peer_id: string;
	avatar_id: number;
	messages: ChatMessage[];
	timestamp: number;
}
