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
	const [autoScroll, setAutoScroll] = useState(false);

	useEffect(() => {
		commands.utilGetLogEntries().then((list) => setLogEntries([...list]));
	}, []);

	// biome-ignore lint/correctness/useExhaustiveDependencies(shouldShowLogLevel): logsShown is necessary
	// biome-ignore lint/correctness/useExhaustiveDependencies(autoScroll): logsShown is necessary
	useEffect(() => {
		(async () => {
			const logLevel = await commands.environmentLogsLevel();
			setShouldShowLogLevel(logLevel);
			const autoScroll = await commands.environmentLogsAutoScroll();
			setAutoScroll(autoScroll);
		})();
	}, [shouldShowLogLevel, autoScroll]);

	const handleLogLevelChange = (newLogLevel: LogLevel[]) => {
		setShouldShowLogLevel(newLogLevel);
		commands.environmentSetLogsLevel(newLogLevel).catch((err) => {
			console.error("Failed to update log level: ", err);
		});
	};

	const handleLogAutoScrollChange = (newAutoScroll: boolean) => {
		setAutoScroll(newAutoScroll);
		commands.environmentSetLogsAutoScroll(newAutoScroll).catch((err) => {
			console.error("Failed to update log auto scroll: ", err);
		});
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
			<HNavBar className={"flex-shrink-0"}>
				<ManageLogsHeading
					search={search}
					setSearch={setSearch}
					shouldShowLogLevel={shouldShowLogLevel}
					handleLogLevelChange={handleLogLevelChange}
					handleLogAutoScrollChange={handleLogAutoScrollChange}
					autoScroll={autoScroll}
				/>
			</HNavBar>
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
		<div
			className={
				"flex flex-wrap flex-shrink-0 flex-grow-0 flex-row gap-2 items-center w-full"
			}
		>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
				{tc("logs")}
			</p>

			<SearchBox
				className={"w-max flex-grow"}
				value={search}
				onChange={(e) => setSearch(e.target.value)}
				ref={searchRef}
			/>

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
		</div>
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
