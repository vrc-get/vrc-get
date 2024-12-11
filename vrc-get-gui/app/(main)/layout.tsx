"use client";

import { SideBar } from "@/components/SideBar";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";

export default function MainLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	const [animationState, setAnimationState] = useState("");
	const [previousPathName, setPreviousPathName] = useState("");
	const [isVisible, setIsVisible] = useState(false);
	const pathName = usePathname();

	useEffect(() => {
		setPreviousPathName(pathName);
	}, [pathName]);

	useEffect(() => {
		if (
			pathName.startsWith("/packages") &&
			!previousPathName.startsWith("/packages/")
		) {
			setAnimationState("fade-in");
		} else if (pathName === "/packages/repositories") {
			setAnimationState("slide-right");
		} else if (pathName === "/packages/user-packages") {
			setAnimationState("slide-left");
		} else if (
			pathName.startsWith("/projects") &&
			!previousPathName.includes("/projects")
		) {
			setAnimationState("fade-in");
		} else if (pathName.startsWith("/projects/")) {
			setAnimationState("slide-left");
		} else {
			setAnimationState("fade-in");
		}
	}, [pathName]);

	useEffect(() => {
		setIsVisible(true);
	}, []);

	return (
		<>
			<SideBar className={`flex-grow-0 ${isVisible ? "slide-right" : ""}`} />
			<div
				className={`h-screen flex-grow overflow-hidden flex p-4 ${animationState}`}
				onAnimationEnd={() => setAnimationState("")}
			>
				{children}
			</div>
		</>
	);
}
