import {SideBar} from "@/components/SideBar";

export default function MainLayout({
																		 children,
																	 }: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<>
			<SideBar className={"flex-grow-0"}/>
			<div className={"h-screen flex-grow overflow-hidden flex p-4"}>
				{children}
			</div>
		</>
);
}
