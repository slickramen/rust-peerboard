import "./Header.css";

interface HeaderProps {
	connected: boolean;
	username: string;
}

const Header = ({ connected, username }: HeaderProps) => {
	return (
		<header className="header">
			<span
				className={
					connected ? "status connected" : "status disconnected"
				}
			>
				{connected ? "connected" : "disconnected"}
			</span>

			<span>{username}</span>
		</header>
	);
};

export default Header;
