"use client";

import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ArrowDownFromLine } from "lucide-react";
import { useRef, useState } from "react";
import { HNavBar, VStack } from "@/components/layout";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { LogEntry, LogLevel } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import globalInfo from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { useSessionStorage } from "@/lib/useSessionStorage";
import { LogsListCard } from "./-logs-list-card";

export const Route = createFileRoute("/_main/log/")({
	component: Page,
});

const utilGetLogEntries = queryOptions({
	queryKey: ["utilGetLogEntries"],
	queryFn: async () => commands.utilGetLogEntries(),
});

const environmentLogsLevel = queryOptions({
	queryKey: ["environmentLogsLevel"],
	queryFn: async () => commands.environmentLogsLevel(),
});

function Page() {
	const [search, setSearch] = useState("");

	const queryClient = useQueryClient();
	const logEntriesQuery = useQuery(utilGetLogEntries);
	const logsLevel = useQuery(environmentLogsLevel);

	const handleLogLevelChange = useMutation({
		mutationFn: async (value: LogLevel[]) =>
			commands.environmentSetLogsLevel(value),
		onMutate: async (value) => {
			await queryClient.cancelQueries(environmentLogsLevel);
			const data = queryClient.getQueryData(environmentLogsLevel.queryKey);
			queryClient.setQueryData(environmentLogsLevel.queryKey, value);
			return data;
		},
		onError: (e, _, data) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentLogsLevel.queryKey, data);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentLogsLevel);
		},
	});

	const autoScroll = useSessionStorage({
		key: "logs_auto_scroll",
		parse: (value) => value === "true",
		fallbackValue: true,
	});

	const handleLogAutoScrollChange = (value: boolean) => {
		sessionStorage.setItem("logs_auto_scroll", String(value));
		// Manually dispatch storage event to force state synchronization within the same page,
		// as native sessionStorage.setItem doesn't trigger storage event for the current origin
		window.dispatchEvent(
			new StorageEvent("storage", {
				key: "logs_auto_scroll",
				newValue: String(value),
				storageArea: sessionStorage,
			}),
		);
	};

	useTauriListen<LogEntry>("log", (event) => {
		const entry = event.payload as LogEntry;
		const entries = queryClient.getQueryData(utilGetLogEntries.queryKey) ?? [];
		queryClient.setQueryData(utilGetLogEntries.queryKey, [...entries, entry]);
	});

	const shouldShowLogLevel = logsLevel.data ?? [];

	return (
		<VStack>
			<ManageLogsHeading
				search={search}
				setSearch={setSearch}
				shouldShowLogLevel={shouldShowLogLevel}
				handleLogLevelChange={handleLogLevelChange.mutate}
				handleLogAutoScrollChange={handleLogAutoScrollChange}
				autoScroll={autoScroll}
			/>
			<main className="shrink overflow-hidden flex w-full h-full">
				<LogsListCard
					logEntry={logEntriesQuery.data ?? []}
					search={search}
					shouldShowLogLevel={shouldShowLogLevel}
					autoScroll={autoScroll}
				/>
			</main>
		</VStack>
	);
}

function ManageLogsHeading({
	search,
	setSearch,
	shouldShowLogLevel,
	handleLogLevelChange,
	handleLogAutoScrollChange,
	autoScroll,
}: {
	search: string;
	setSearch: (value: string) => void;
	shouldShowLogLevel: LogLevel[];
	handleLogLevelChange: (newLogLevels: LogLevel[]) => void;
	handleLogAutoScrollChange: (newAutoScroll: boolean) => void;
	autoScroll: boolean;
}) {
	const searchRef = useRef<HTMLInputElement>(null);

	useDocumentEvent(
		"keydown",
		(e) => {
			if (isFindKey(e)) {
				searchRef.current?.focus();
			}
		},
		[],
	);

	return (
		<HNavBar
			className={"shrink-0 compact:py-0.5"}
			trailingClassName="compact:-mr-2.5"
			leading={
				<>
					<p className="cursor-pointer py-1.5 font-bold grow-0">{tc("logs")}</p>

					<SearchBox
						className={"w-max grow"}
						inputClassName={"compact:h-10"}
						iconClassName={"compact:top-[17px]"}
						value={search}
						onChange={(e) => setSearch(e.target.value)}
						ref={searchRef}
					/>
				</>
			}
			trailing={
				<>
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button className={"shrink-0 p-3 compact:h-10"}>
								{tc("logs:manage:select logs level")}
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent>
							<LogLevelMenuItem
								logLevel="Info"
								shouldShowLogLevel={shouldShowLogLevel}
								handleLogLevelChange={handleLogLevelChange}
							/>
							<LogLevelMenuItem
								logLevel="Warn"
								className="text-warning"
								shouldShowLogLevel={shouldShowLogLevel}
								handleLogLevelChange={handleLogLevelChange}
							/>
							<LogLevelMenuItem
								logLevel="Error"
								className="text-destructive"
								shouldShowLogLevel={shouldShowLogLevel}
								handleLogLevelChange={handleLogLevelChange}
							/>
							<LogLevelMenuItem
								logLevel="Debug"
								className="text-info"
								shouldShowLogLevel={shouldShowLogLevel}
								handleLogLevelChange={handleLogLevelChange}
							/>
							{/* Currently no trace level logs will be passed to frontend */}
							{/*<LogLevelMenuItem
                        logLevel="Trace"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
                    />*/}
						</DropdownMenuContent>
					</DropdownMenu>

					<Button
						className={"compact:h-10"}
						onClick={() =>
							commands.utilOpen(
								`${globalInfo.vpmHomeFolder}/vrc-get/gui-logs`,
								"ErrorIfNotExists",
							)
						}
					>
						{tc("settings:button:open logs")}
					</Button>

					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant={"ghost"}
								onClick={() => handleLogAutoScrollChange(!autoScroll)}
								className={`compact:h-10 ${
									autoScroll
										? "bg-secondary border border-primary"
										: "bg-transparent"
								}`}
							>
								<ArrowDownFromLine className={"w-5 h-5"} />
							</Button>
						</TooltipTrigger>
						<TooltipContent>{tc("logs:manage:auto scroll")}</TooltipContent>
					</Tooltip>
				</>
			}
		/>
	);
}

function LogLevelMenuItem({
	logLevel,
	className,
	shouldShowLogLevel,
	handleLogLevelChange,
}: {
	logLevel: LogLevel;
	className?: string;
	shouldShowLogLevel: LogLevel[];
	handleLogLevelChange: (newLogLevels: LogLevel[]) => void;
}) {
	const selected = shouldShowLogLevel.includes(logLevel);

	const onChange = () => {
		const newLogLevels = selected
			? shouldShowLogLevel.filter(
					(logLevelFilter) => logLevelFilter !== logLevel,
				)
			: [...shouldShowLogLevel, logLevel];

		handleLogLevelChange(newLogLevels);
	};

	return (
		<DropdownMenuItem
			className="p-0"
			onSelect={(e) => {
				e.preventDefault();
			}}
		>
			<label
				className={
					"flex cursor-pointer items-center gap-2 p-2 whitespace-normal"
				}
			>
				<Checkbox
					checked={selected}
					onCheckedChange={onChange}
					className="hover:before:content-none"
				/>
				<p className={className}>{logLevel}</p>
			</label>
		</DropdownMenuItem>
	);
}
