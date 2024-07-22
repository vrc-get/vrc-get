"use client";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	environmentClearSetupProcess,
	utilIsBadHostname,
} from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastNormal } from "@/lib/toast";
import { useQuery } from "@tanstack/react-query";
import {
	AlignLeft,
	CircleAlert,
	List,
	Package,
	Settings,
	SwatchBook,
} from "lucide-react";
import { useRouter } from "next/navigation";
import type React from "react";

export function SideBar({ className }: { className?: string }) {
	"use client";

	const globalInfo = useGlobalInfo();

	const isBadHostName = useQuery({
		queryKey: ["util_is_bad_hostname"],
		queryFn: utilIsBadHostname,
		refetchOnMount: false,
		refetchOnReconnect: false,
		refetchOnWindowFocus: false,
		refetchInterval: false,
		initialData: false,
	});

	const copyVersionName = () => {
		if (globalInfo.version != null) {
			void navigator.clipboard.writeText(globalInfo.version);
			toastNormal(tc("sidebar:toast:version copied"));
		}
	};
	const isDev = process.env.NODE_ENV === "development";

	return (
		<Card
			className={`${className} flex w-auto max-w-80 p-2 shadow-xl shadow-primary/5 ml-4 my-4 shrink-0 overflow-auto`}
		>
			<div className="flex flex-col gap-1 p-2 min-w-40 flex-grow">
				<SideBarItem href={"/projects"} text={tc("projects")} icon={List} />
				<SideBarItem
					href={"/packages/repositories"}
					text={tc("packages")}
					icon={Package}
				/>
				<SideBarItem href={"/settings"} text={tc("settings")} icon={Settings} />
				<SideBarItem href={"/log"} text={tc("logs")} icon={AlignLeft} />
				{isDev && <DevRestartSetupButton />}
				{isDev && (
					<SideBarItem
						href={"/settings/palette"}
						text={"UI Palette (dev only)"}
						icon={SwatchBook}
					/>
				)}
				<div className={"flex-grow"} />
				{isBadHostName.data && <BadHostNameDialogButton />}
				<Button
					variant={"ghost"}
					className={
						"text-sm justify-start hover:bg-card hover:text-card-foreground"
					}
					onClick={copyVersionName}
				>
					{globalInfo.version ? `v${globalInfo.version}` : "unknown"}
				</Button>
			</div>
		</Card>
	);
}

function SideBarItem({
	href,
	text,
	icon,
}: {
	href: string;
	text: React.ReactNode;
	icon: React.ComponentType<{ className?: string }>;
}) {
	const router = useRouter();
	const IconElenment = icon;
	return (
		<Button
			variant={"ghost"}
			className={"justify-start flex-shrink-0"}
			onClick={() => router.push(href)}
		>
			<div className={"mr-4"}>
				<IconElenment className="h-5 w-5" />
			</div>
			{text}
		</Button>
	);
}

function BadHostNameDialogButton() {
	return (
		<Dialog>
			<DialogTrigger asChild>
				<Button
					variant={"ghost"}
					className={
						"text-sm justify-start hover:bg-card hover:text-warning text-warning"
					}
				>
					<div className={"mr-4"}>
						<CircleAlert className="h-5 w-5" />
					</div>
					{tc("sidebar:bad hostname")}
				</Button>
			</DialogTrigger>
			<DialogContent className={"max-w-[50vw]"}>
				<DialogHeader>
					<h1 className={"text-warning text-center"}>
						{tc("sidebar:dialog:bad hostname")}
					</h1>
				</DialogHeader>
				<DialogDescription className={"whitespace-normal"}>
					{tc("sidebar:dialog:bad hostname description")}
				</DialogDescription>
				<DialogFooter>
					<DialogClose asChild>
						<Button>{tc("general:button:close")}</Button>
					</DialogClose>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function DevRestartSetupButton() {
	const router = useRouter();
	const onClick = async () => {
		await environmentClearSetupProcess();
		router.push("/setup/appearance");
	};
	return (
		<Button
			variant={"ghost"}
			className={"justify-start flex-shrink-0"}
			onClick={onClick}
		>
			<div className={"mr-4"}>
				<Settings className="h-5 w-5" />
			</div>
			Restart Setup (dev only)
		</Button>
	);
}
