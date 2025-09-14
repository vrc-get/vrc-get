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
	Info,
	List,
	Package,
	Settings,
	SwatchBook,
} from "lucide-react";
import type React from "react";
import {
	GuiAnimationSwitch,
	GuiCompactSwitch,
	ThemeSelector,
} from "@/components/common-setting-parts";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
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
			className={`${className} flex w-auto max-w-80 p-2 shadow-xl shadow-primary/5 ml-4 my-4 shrink-0 overflow-auto compact:p-0 compact:ml-2 compact:my-2`}
		>
			<div className="flex flex-col gap-1 p-2 min-w-40 grow compact:min-w-0">
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
				{isDev && <StyleQuickAccess />}
				<div className={"grow"} />
				{isBadHostName.data && <BadHostNameDialogButton />}
				<SideBarButton
					icon={Info}
					showIconOnlyWhenCompact
					className="hover:bg-card"
					onClick={copyVersionName}
				>
					{globalInfo.version ? `v${globalInfo.version}` : "unknown"}
				</SideBarButton>
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
	const getFirstPathSegment = (path: string) => {
		return path.split("/")[1] || "";
	};
	const isActive =
		getFirstPathSegment(location.pathname || "") === getFirstPathSegment(href);
	return (
		<SideBarButton
			icon={icon}
			className={
				isActive ? "bg-secondary border border-primary" : "bg-transparent"
			}
			onClick={() => navigate({ to: href })}
		>
			{text}
		</SideBarButton>
	);
}

function BadHostNameDialogButton() {
	return (
		<Dialog>
			<DialogTrigger asChild>
				<SideBarButton
					icon={CircleAlert}
					className="text-warning hover:bg-card hover:text-warning"
				>
					{tc("sidebar:bad hostname")}
				</SideBarButton>
			</DialogTrigger>
			<DialogContent className={"max-w-[50vw]"}>
				<DialogHeader>
					<h1 className={"text-warning text-center"}>
						{tc("sidebar:dialog:bad hostname")}
					</h1>
				</DialogHeader>
				<div className={"whitespace-normal"}>
					{tc("sidebar:dialog:bad hostname description")}
				</div>
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
		<SideBarButton icon={Settings} onClick={onClick}>
			Restart Setup (dev only)
		</SideBarButton>
	);
}

function SideBarButton({
	icon,
	showIconOnlyWhenCompact,
	className,
	children,
	...props
}: {
	icon: React.ComponentType<{ className?: string }>;
	showIconOnlyWhenCompact?: boolean;
	className?: string;
	children: React.ReactNode;
} & React.ComponentProps<typeof Button>) {
	const IconElement = icon;
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant="ghost"
					className={`justify-start ${className} compact:justify-center compact:px-3 compact:size-10`}
					{...props}
				>
					<div
						className={`mr-4 compact:mr-0 ${showIconOnlyWhenCompact ? "hidden compact:block" : ""}`}
					>
						<IconElement className="h-5 w-5" />
					</div>
					<span className="compact:hidden">{children}</span>
				</Button>
			</TooltipTrigger>
			<TooltipContent side="right">{children}</TooltipContent>
		</Tooltip>
	);
}

export function StyleQuickAccess() {
	return (
		<Popover>
			<PopoverTrigger asChild>
				<SideBarButton icon={SwatchBook}>
					Style Settings (dev only)
				</SideBarButton>
			</PopoverTrigger>
			<PopoverContent>
				<ThemeSelector />
				<GuiAnimationSwitch />
				<GuiCompactSwitch />
			</PopoverContent>
		</Popover>
	);
}
