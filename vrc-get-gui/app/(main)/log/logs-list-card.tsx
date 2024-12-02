import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ScrollArea } from "@/components/ui/scroll-area";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { type LogEntry, type LogLevel, commands } from "@/lib/bindings";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import globalInfo from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { BugOff, CircleX, Info, OctagonAlert } from "lucide-react";
import { ArrowDownFromLine } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";

export const LogsListCard = memo(function LogsListCard({
	logEntry,
}: {
	logEntry: LogEntry[];
}) {
	const [search, setSearch] = useState("");
	const [shouldShowLogLevel, setShouldShowLogLevel] = useState<LogLevel[]>([
		"Info",
		"Warn",
		"Error",
	]);
	const [autoScroll, setAutoScroll] = useState(true);

	const logsShown = useMemo(
		() =>
			logEntry.filter(
				(log) =>
					log.message.toLowerCase().includes(search?.toLowerCase() ?? "") &&
					shouldShowLogLevel.includes(log.level),
			),
		[logEntry, search, shouldShowLogLevel],
	);

	const scrollContainerRef = useRef<React.ElementRef<typeof ScrollArea>>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies(logsShown): logsShown is necessary
	useEffect(() => {
		if (autoScroll && scrollContainerRef.current) {
			scrollContainerRef.current.scrollTop =
				scrollContainerRef.current.scrollHeight;
		}
	}, [logsShown, autoScroll]);

	const TABLE_HEAD = ["logs:time", "logs:level", "logs:message"];

	return (
		<Card className="flex-grow flex-shrink flex shadow-none w-full">
			<CardContent className="w-full p-2 flex flex-col gap-2">
				<ManageLogsHeading
					search={search}
					setSearch={setSearch}
					shouldShowLogLevel={shouldShowLogLevel}
					setShouldShowLogLevel={setShouldShowLogLevel}
					setAutoScroll={(value) => setAutoScroll(value)}
					autoScroll={autoScroll}
				/>
				<ScrollableCardTable className={"h-full"} ref={scrollContainerRef}>
					<thead>
						<tr>
							{TABLE_HEAD.map((head, index) => (
								<th
									// biome-ignore lint/suspicious/noArrayIndexKey: static array
									key={index}
									className={
										"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
									}
								>
									<small className="font-normal leading-none">{tc(head)}</small>
								</th>
							))}
							<th
								className={
									"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
								}
							/>
						</tr>
					</thead>
					<tbody>
						{logsShown.map((row) => (
							<tr key={row.time} className="even:bg-secondary/30">
								<LogRow log={row} />
							</tr>
						))}
					</tbody>
				</ScrollableCardTable>
			</CardContent>
		</Card>
	);
});

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
				"flex flex-wrap flex-shrink-0 flex-grow-0 flex-row gap-2 items-center"
			}
		>
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
						className={autoScroll ? "bg-secondary" : "bg-transparent"}
					>
						<ArrowDownFromLine className={"w-5 h-5"} />
					</Button>
				</TooltipTrigger>
				<TooltipContent>{tc("logs:manage:auto scroll")}</TooltipContent>
			</Tooltip>
		</div>
	);
}

const LogRow = memo(function LogRow({
	log,
}: {
	log: LogEntry;
}) {
	const cellClass = "p-2.5";

	const formatDate = (dateString: string) => {
		const date = new Date(dateString);
		return date.toLocaleString();
	};

	const getFontColorClass = (level: LogLevel) => {
		switch (level) {
			case "Info":
				return "";
			case "Warn":
				return "text-warning";
			case "Error":
				return "text-destructive";
			case "Debug":
				return "text-info";
			default:
				return "";
		}
	};

	const fontColorClass = getFontColorClass(log.level);
	const typeIconClass = `${fontColorClass} w-5 h-5`;

	return (
		<>
			<td className={`${cellClass} min-w-32 w-32`}>{formatDate(log.time)}</td>
			<td className={`${cellClass} min-w-28 w-28`}>
				<div className="flex flex-row gap-2">
					<div className="flex items-center">
						{log.level === "Info" ? (
							<Info className={typeIconClass} />
						) : log.level === "Warn" ? (
							<OctagonAlert className={typeIconClass} />
						) : log.level === "Error" ? (
							<CircleX className={typeIconClass} />
						) : log.level === "Debug" ? (
							<BugOff className={typeIconClass} />
						) : (
							<Info className={typeIconClass} />
						)}
					</div>
					<div className="flex flex-col justify-center">
						<p className={`font-normal ${fontColorClass}`}>{log.level}</p>
					</div>
				</div>
			</td>
			<td className={`${cellClass} min-w-32 w-32`}>{log.message}</td>
		</>
	);
});
