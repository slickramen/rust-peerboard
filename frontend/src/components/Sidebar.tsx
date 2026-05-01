import "./Sidebar.css";

const Channel = () => {
	return <div className="sidebar-channel">ge</div>;
};

export const Sidebar = () => {
	return (
		<div className="sidebar-base">
			<div className="sidebar-stack">
				<Channel />
			</div>
		</div>
	);
};

export const RightSideBar = () => {
	return <div className="sidebar-base right-side"></div>;
};
