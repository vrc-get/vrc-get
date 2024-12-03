import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import type { ScrollArea } from "@/components/ui/scroll-area";
import type { LogEntry, LogLevel } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { BugOff, CircleX, Info, OctagonAlert } from "lucide-react";
import { memo, useEffect, useMemo, useRef } from "react";

export const LogsListCard = memo(function LogsListCard({
	logEntry,
	shouldShowLogLevel,
	search,
	autoScroll,
}: {
	logEntry: LogEntry[];
	shouldShowLogLevel: LogLevel[];
	search: string;
	autoScroll: boolean;
}) {
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
	);
});

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
