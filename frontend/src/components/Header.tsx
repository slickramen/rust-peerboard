import "./Header.css";

interface HeaderProps {
	connected: boolean;
	username: string;
	avatar_id: number;
}

const Header = ({ connected, username, avatar_id }: HeaderProps) => {
	return (
		<header className="header">
			<span
				className={
					connected ? "status connected" : "status disconnected"
				}
			>
				<div className="connection-orb" />
				{connected ? "online" : "offline"}
			</span>

			{connected && (
				<div className="avatar-wrapper">
					<strong>{username}</strong>
					<div className="avatar">
						<img src={`/icons/${avatar_id}.png`}></img>
					</div>
				</div>
			)}
		</header>
	);
};

export default Header;
