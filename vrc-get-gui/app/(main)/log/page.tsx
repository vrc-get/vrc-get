"use client";

import { HNavBar, VStack } from "@/components/layout";
import type { LogEntry, LogLevel } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { useTauriListen } from "@/lib/use-tauri-listen";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { LogsListCard } from "./logs-list-card";
import { SearchBox } from "@/components/SearchBox";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ArrowDownFromLine } from "lucide-react";
import globalInfo from "@/lib/global-info";
import { isFindKey, useDocumentEvent } from "@/lib/events";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);
	
	const [search, setSearch] = useState("");
	const [shouldShowLogLevel, setShouldShowLogLevel] = useState<LogLevel[]>([
		"Info",
		"Warn",
		"Error",
	]);
	const [autoScroll, setAutoScroll] = useState(true);

	useEffect(() => {
		commands.utilGetLogEntries().then((list) => setLogEntries([...list]));
	}, []);

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
					setShouldShowLogLevel={setShouldShowLogLevel}
					setAutoScroll={(value) => setAutoScroll(value)}
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
	setShouldShowLogLevel,
	setAutoScroll,
	autoScroll,
}: {
	search: string;
	setSearch: (value: string) => void;
	shouldShowLogLevel: LogLevel[];
	setShouldShowLogLevel: React.Dispatch<React.SetStateAction<LogLevel[]>>;
	setAutoScroll: React.Dispatch<React.SetStateAction<boolean>>;
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
						setShouldShowLogLevel={setShouldShowLogLevel}
					/>
					<LogLevelMenuItem
						logLevel="Warn"
						className="text-warning"
						shouldShowLogLevel={shouldShowLogLevel}
						setShouldShowLogLevel={setShouldShowLogLevel}
					/>
					<LogLevelMenuItem
						logLevel="Error"
						className="text-destructive"
						shouldShowLogLevel={shouldShowLogLevel}
						setShouldShowLogLevel={setShouldShowLogLevel}
					/>
					<LogLevelMenuItem
						logLevel="Debug"
						className="text-info"
						shouldShowLogLevel={shouldShowLogLevel}
						setShouldShowLogLevel={setShouldShowLogLevel}
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
						onClick={() => setAutoScroll((prev) => !prev)}
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
	setShouldShowLogLevel,
}: {
	logLevel: LogLevel;
	className?: string;
	shouldShowLogLevel: LogLevel[];
	setShouldShowLogLevel: React.Dispatch<React.SetStateAction<LogLevel[]>>;
}) {
	const selected = shouldShowLogLevel.includes(logLevel);
	const onChange = () => {
		if (selected) {
			setShouldShowLogLevel((prev) =>
				prev.filter((logLevelFilter) => logLevelFilter !== logLevel),
			);
		} else {
			setShouldShowLogLevel((prev) => [...prev, logLevel]);
		}
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
