"use client";

import { useRouter } from "next/navigation";

export default function SetupLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	const isDev = process.env.NODE_ENV == "development";

	return (
		<>
			<div className={"h-screen flex-grow overflow-hidden flex p-4"}>
				{children}
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
				<button onClick={() => router.back()}>Go Back</button>
				<button onClick={() => router.push("/settings")}>Go Settings</button>
			</div>
		</div>
	);
}
