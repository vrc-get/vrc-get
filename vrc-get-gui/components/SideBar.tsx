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
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastNormal } from "@/lib/toast";

export function SideBar({ className, compact }: { className?: string, compact?: boolean }) {
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
			className={`${className} flex w-auto max-w-80 ${compact ? "px-0 py-2" : "p-2"} shadow-xl shadow-primary/5 ml-4 my-4 shrink-0 overflow-auto`}
		>
			<div className={`flex flex-col gap-1 p-2 ${compact ? "min-w-0" : "min-w-40"} grow`}>
				<SideBarItem href={"/projects"} text={tc("projects")} icon={List} compact={compact} />
				<SideBarItem
					href={"/packages/repositories"}
					text={tc("resources")}
					icon={Package}
					compact={compact}
				/>
				<SideBarItem href={"/settings"} text={tc("settings")} icon={Settings} compact={compact} />
				<SideBarItem href={"/log"} text={tc("logs")} icon={AlignLeft} compact={compact} />
				{isDev && <DevRestartSetupButton compact={compact} />}
				{isDev && (
					<SideBarItem
						href={"/dev-palette"}
						text={"UI Palette (dev only)"}
						icon={SwatchBook}
						compact={compact}
					/>
				)}
				<div className={"grow"} />
				{isBadHostName.data && <BadHostNameDialogButton compact={compact} />}
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							variant={"ghost"}
							className={
								"text-sm justify-start hover:bg-card hover:text-card-foreground"
							}
							onClick={copyVersionName}
						>
							{compact ? "ver" : globalInfo.version ? `v${globalInfo.version}` : "unknown"}
						</Button>
					</TooltipTrigger>
					<TooltipContent side="right">{globalInfo.version ? `v${globalInfo.version}` : "unknown"}</TooltipContent>
				</Tooltip>
			</div>
		</Card>
	);
}

function SideBarItem({
	href,
	text,
	icon,
	compact,
}: {
	href: keyof RegisteredRouter["routesByPath"];
	text: React.ReactNode;
	icon: React.ComponentType<{ className?: string }>;
	compact?: boolean;
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
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant={"ghost"}
					className={`justify-start shrink-0 ${isActive ? "bg-secondary border border-primary" : "bg-transparent"}`}
					onClick={() => navigate({ to: href })}
				>
					<div className={compact ? "mr-0" : "mr-4"}>
						<IconElenment className="h-5 w-5" />
					</div>
					{compact ? "" : text}
				</Button>
			</TooltipTrigger>
			<TooltipContent side="right">{text}</TooltipContent>
		</Tooltip>
	);
}

function BadHostNameDialogButton({ compact }: { compact?: boolean }) {
	return (
		<Dialog>
			<Tooltip>
				<DialogTrigger asChild>
					<TooltipTrigger asChild>
						<Button
							variant={"ghost"}
							className={
								"text-sm justify-start hover:bg-card hover:text-warning text-warning"
							}
						>
							<div className={compact ? "mr-0" : "mr-4"}>
								<CircleAlert className="h-5 w-5" />
							</div>
							{compact ? "" : tc("sidebar:bad hostname")}
						</Button>
					</TooltipTrigger>
				</DialogTrigger>
				<TooltipContent side="right">{tc("sidebar:bad hostname")}</TooltipContent>
			</Tooltip>
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

function DevRestartSetupButton({ compact }: { compact?: boolean }) {
	const navigate = useNavigate();
	const onClick = async () => {
		await commands.environmentClearSetupProcess();
		navigate({ to: "/setup/appearance" });
	};
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant={"ghost"}
					className={"justify-start shrink-0"}
					onClick={onClick}
				>
					<div className={compact ? "mr-0" : "mr-4"}>
						<Settings className="h-5 w-5" />
					</div>
					{compact ? "" : "Restart Setup (dev only)"}
				</Button>
			</TooltipTrigger>
			<TooltipContent side="right">{"Restart Setup (dev only)"}</TooltipContent>
		</Tooltip>
	);
}
