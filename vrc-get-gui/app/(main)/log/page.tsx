"use client";

import { SearchBox } from "@/components/SearchBox";
import { HNavBar, VStack } from "@/components/layout";
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
import { useTauriListen } from "@/lib/use-tauri-listen";
import { ArrowDownFromLine } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { LogsListCard } from "./logs-list-card";

export default function Page() {
	const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
	const [search, setSearch] = useState("");
	const [shouldShowLogLevel, setShouldShowLogLevel] = useState<LogLevel[]>([]);
	const [autoScroll, setAutoScroll] = useState(true);

	useEffect(() => {
		commands.utilGetLogEntries().then(setLogEntries);
		commands.environmentLogsLevel().then(setShouldShowLogLevel);
		const logsAutoScroll =
			sessionStorage.getItem("logs_auto_scroll") === "true";
		setAutoScroll(logsAutoScroll);
	}, []);

	const handleLogLevelChange = (value: LogLevel[]) => {
		setShouldShowLogLevel(value);
		commands.environmentSetLogsLevel(value).catch((err) => {
			console.error("Failed to update log level: ", err);
		});
	};

	const handleLogAutoScrollChange = (value: boolean) => {
		sessionStorage.setItem("logs_auto_scroll", String(value));
		setAutoScroll(value);
	};

	useTauriListen<LogEntry>(
		"log",
		useCallback((event) => {
			setLogEntries((entries) => {
				const entry = event.payload as LogEntry;
				return [...entries, entry];
			});
		}, []),
	);

	return (
		<VStack>
			<ManageLogsHeading
				search={search}
				setSearch={setSearch}
				shouldShowLogLevel={shouldShowLogLevel}
				handleLogLevelChange={handleLogLevelChange}
				handleLogAutoScrollChange={handleLogAutoScrollChange}
				autoScroll={autoScroll}
			/>
			<main className="flex-shrink overflow-hidden flex w-full h-full">
				<LogsListCard
					logEntry={logEntries}
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
			className={"flex-shrink-0"}
			leading={
				<>
					<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
						{tc("logs")}
					</p>

					<SearchBox
						className={"w-max flex-grow"}
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
							<Button className={"flex-shrink-0 p-3"}>
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
								className={
									autoScroll
										? "bg-secondary border border-primary"
										: "bg-transparent"
								}
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
