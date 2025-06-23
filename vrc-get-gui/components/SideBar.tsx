"use client";

import { useQuery } from "@tanstack/react-query";
import {
	type RegisteredRouter,
	useLocation,
	useNavigate,
} from "@tanstack/react-router";
import {
	AlignLeft,
	CircleAlert,
	List,
	Package,
	Settings,
	SwatchBook,
} from "lucide-react";
import type React from "react";
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
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastNormal } from "@/lib/toast";

export function SideBar({ className }: { className?: string }) {
	"use client";

	const globalInfo = useGlobalInfo();

	const isBadHostName = useQuery({
		queryKey: ["util_is_bad_hostname"],
		queryFn: commands.utilIsBadHostname,
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
	const isDev = import.meta.env.DEV;

	return (
		<Card
			className={`${className} flex w-auto max-w-80 p-2 shadow-xl shadow-primary/5 ml-4 my-4 shrink-0 overflow-auto`}
		>
			<div className="flex flex-col gap-1 p-2 min-w-40 grow">
				<SideBarItem href={"/projects"} text={tc("projects")} icon={List} />
				<SideBarItem
					href={"/packages/repositories"}
					text={tc("resources")}
					icon={Package}
				/>
				<SideBarItem href={"/settings"} text={tc("settings")} icon={Settings} />
				<SideBarItem href={"/log"} text={tc("logs")} icon={AlignLeft} />
				{isDev && <DevRestartSetupButton />}
				{isDev && (
					<SideBarItem
						href={"/dev-palette"}
						text={"UI Palette (dev only)"}
						icon={SwatchBook}
					/>
				)}
				<div className={"grow"} />
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
	href: keyof RegisteredRouter["routesByPath"];
	text: React.ReactNode;
	icon: React.ComponentType<{ className?: string }>;
}) {
	const location = useLocation();
	const navigate = useNavigate();
	const IconElenment = icon;
	const getFirstPathSegment = (path: string) => {
		return path.split("/")[1] || "";
	};
	const isActive =
		getFirstPathSegment(location.pathname || "") === getFirstPathSegment(href);
	return (
		<Button
			variant={"ghost"}
			className={`justify-start shrink-0 ${isActive ? "bg-secondary border border-primary" : "bg-transparent"}`}
			onClick={() => navigate({ to: href })}
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
	const navigate = useNavigate();
	const onClick = async () => {
		await commands.environmentClearSetupProcess();
		navigate({ to: "/setup/appearance" });
	};
	return (
		<Button
			variant={"ghost"}
			className={"justify-start shrink-0"}
			onClick={onClick}
		>
			<div className={"mr-4"}>
				<Settings className="h-5 w-5" />
			</div>
			Restart Setup (dev only)
		</Button>
	);
}
