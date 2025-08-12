"use client";

import { createFileRoute, Outlet, useLocation } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { SideBar } from "@/components/SideBar";
import { commands } from "@/lib/bindings";
import { useDocumentEvent } from "@/lib/events";
import { updateCurrentPath, usePrevPathName } from "@/lib/prev-page";
import { useEffectEvent } from "@/lib/use-effect-event";

export const Route = createFileRoute("/_main")({
	component: MainLayout,
});

function MainLayout() {
	const [animationState, setAnimationState] = useState("");
	const [isVisible, setIsVisible] = useState(false);
	const [guiAnimation, setGuiAnimation] = useState(false);
	const previousPathName = usePrevPathName();
	const pathName = useLocation().pathname;

	useDocumentEvent(
		"gui-animation",
		(event) => {
			setGuiAnimation(event.detail);
		},
		[],
	);

	const onPathChange = useEffectEvent((pathName: string) => {
		updateCurrentPath(pathName);

		(async () => {
			setGuiAnimation(await commands.environmentGuiAnimation());
		})();

		if (!guiAnimation) return;

		if (pathName === previousPathName) return;
		const pageCategory = pathName.split("/")[1];
		const previousPageCategory = previousPathName.split("/")[1];
		if (pageCategory !== previousPageCategory) {
			// category change is always fade-in
			setAnimationState("fade-in");
		} else {
			// go deeper is slide-left, go back is slide-right, and no animation if not child-parent relation
			if (pathName.startsWith(previousPathName)) {
				setAnimationState("slide-left");
			} else if (previousPathName.startsWith(pathName)) {
				setAnimationState("slide-right");
			}
		}
	});

	useEffect(() => {
		onPathChange(pathName);
	}, [pathName]);

	useEffect(() => {
		(async () => {
			if (await commands.environmentGuiCompact())
			{
				document.documentElement.setAttribute("compact", "");
			}
			else
			{
				document.documentElement.removeAttribute("compact");
			}
			setIsVisible(true);
		})();
	}, []);

	return (
		<>
			<SideBar className={`grow-0 ${isVisible ? "slide-right" : "invisible"}`} />
			<div
				className={`h-screen grow overflow-hidden flex p-4 ${animationState}`}
				onAnimationEnd={() => setAnimationState("")}
			>
				<Outlet />
			</div>
		</>
	);
}
