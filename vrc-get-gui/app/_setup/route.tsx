"use client";

import { createFileRoute } from "@tanstack/react-router";
import { useRouter } from "next/navigation";

export const Route = createFileRoute("/_setup")({
	component: SetupLayout,
});

export default function SetupLayout() {
	const isDev = import.meta.env.DEV;

	return (
		<>
			<div className={"h-screen grow overflow-hidden flex p-4"}>
				<Outlet />
			</div>
			{isDev && <DevTools />}
		</>
	);
}

function DevTools() {
	const router = useRouter();
	return (
		<div className={"absolute bottom-0 left-0 p-4 flex flex-col gap-3"}>
			<p>debug tools</p>
			<div className={"flex gap-3"}>
				<button type="button" onClick={() => router.back()}>
					Go Back
				</button>
				<button type="button" onClick={() => router.push("/settings")}>
					Go Settings
				</button>
			</div>
		</div>
	);
}
