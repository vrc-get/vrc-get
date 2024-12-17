"use client";

import { SideBar } from "@/components/SideBar";
import { commands } from "@/lib/bindings";
import { useDocumentEvent } from "@/lib/events";
import { useEffectEvent } from "@/lib/use-effect-event";
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
	const [guiAnimation, setGuiAnimation] = useState(false);
	const pathName = usePathname();

	useDocumentEvent(
		"gui-animation",
		(event) => {
			setGuiAnimation(event.detail);
		},
		[],
	);

	const onPathChange = useEffectEvent((pathName: string) => {
		setPreviousPathName(pathName);

		(async () => {
			setGuiAnimation(await commands.environmentGuiAnimation());
		})();

		if (!guiAnimation) return;

		if (
			pathName.startsWith("/packages") &&
			!previousPathName.startsWith("/packages/")
		) {
			setAnimationState("fade-in");
		} else if (
			pathName.startsWith("/projects") &&
			!previousPathName.startsWith("/projects")
		) {
			setAnimationState("fade-in");
		} else if (pathName.startsWith("/projects/")) {
			setAnimationState("slide-left");
		} else {
			setAnimationState("fade-in");
		}
	});

	useEffect(() => {
		onPathChange(pathName);
	}, [pathName, onPathChange]);

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
