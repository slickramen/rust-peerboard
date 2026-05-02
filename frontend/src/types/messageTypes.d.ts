type WsMessageType =
	| "init"
	| "chat"
	| "subscribe"
	| "unsubscribe"
	| "subscribed"
	| "unsubscribed"
	| "error";

interface BaseMessage {
	type: WsMessageType;
}

interface InitMessage extends BaseMessage {
	type: "init";
	user_id: string;
	username: string;
	topics: string[];
}

interface ErrorMessage extends BaseMessage {
	type: "error";
	message: string;
}

interface SubscribedMessage extends BaseMessage {
	type: "subscribed";
	topic: string;
}

interface UnsubscribedMessage extends BaseMessage {
	type: "unsubscribed";
	topic: string;
}

export interface ChatMessage extends BaseMessage {
	type: "chat";
	peer_id: string;
	nickname: string;
	content: string;
	timestamp: number;
	message_id: string;
	topic: string;
}

type ServerMessage =
	| InitMessage
	| ChatMessage
	| SubscribedMessage
	| UnsubscribedMessage
	| ErrorMessage;

export interface LocalUser {
	user_id: string;
	username: string;
}
