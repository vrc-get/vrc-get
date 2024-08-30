import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { commands, LogEntry } from "@/lib/bindings";
import globalInfo from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { BugOff, CircleX, Info, OctagonAlert } from "lucide-react";
import { memo, useMemo, useState } from "react";

export const LogListCard = memo(function LogListCard({
    logEntry
}: {
    logEntry: LogEntry[]
}) {
    const [search, setSearch] = useState("");
    const [shouldShowLogLevel, setShouldShowLogLevel] = useState([
        "Info",
        "Warn",
        "Error",
    ]);

    const logsShown = useMemo(() => 
        logEntry.filter((log) =>
            log.message.toLowerCase().includes(search?.toLowerCase() ?? "") &&
            shouldShowLogLevel.includes(log.level)
        ), [logEntry, search, shouldShowLogLevel]);

    const TABLE_HEAD = [
		"logs:time",
		"logs:log level",
		"logs:message",
	];

    return (
		<Card className="flex-grow flex-shrink flex shadow-none w-full">
            <CardContent className="w-full p-2 flex flex-col gap-2">
                <ManageLogsHeading 
                    search={search} 
                    setSearch={setSearch}
                    shouldShowLogLevel={shouldShowLogLevel}
                    setShouldShowLogLevel={setShouldShowLogLevel}
                    />
				<ScrollableCardTable>
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
							<tr className="even:bg-secondary/30">
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
    shouldShowLogLevel,
    setShouldShowLogLevel,
}: {
	logLevel: string;
    shouldShowLogLevel: string[];
    setShouldShowLogLevel: React.Dispatch<React.SetStateAction<string[]>>;
}) {
	const selected = shouldShowLogLevel.includes(logLevel);
	const onChange = () => {
		if (selected) {
            setShouldShowLogLevel((prev) =>
              prev.filter((logLevelFilter) => logLevelFilter !== logLevel)
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
				{logLevel}
			</label>
		</DropdownMenuItem>
	);
}

function ManageLogsHeading({
	search,
	setSearch,
    shouldShowLogLevel,
    setShouldShowLogLevel,
}: {
	search: string;
	setSearch: (value: string) => void;
    shouldShowLogLevel: string[];
    setShouldShowLogLevel: React.Dispatch<React.SetStateAction<string[]>>;
}) {
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
			/>

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button className={"flex-shrink-0 p-3"}>
						{tc("logs:manage:select logs level")}
					</Button>
				</DropdownMenuTrigger>
                <DropdownMenuContent className={"max-h-96 w-64"}>
					<LogLevelMenuItem
                        logLevel="Info"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
					/>
                    <LogLevelMenuItem
                        logLevel="Warn"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
					/>
                    <LogLevelMenuItem
                        logLevel="Error"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
					/>
                    <LogLevelMenuItem
                        logLevel="Debug"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
					/>
                    <LogLevelMenuItem
                        logLevel="Trace"
                        shouldShowLogLevel={shouldShowLogLevel}
                        setShouldShowLogLevel={setShouldShowLogLevel}
					/>
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
		</div>
	);

}

const LogRow = memo(function LogRow({
	log,
}: {
	log: LogEntry;
}) {
    const cellClass = "p-2.5";
    const typeIconClass = "w-5 h-5";

    const formatDate = (dateString: string) => {
        const date = new Date(dateString);
        return date.toLocaleString()
    };
    
    return (
		<>
			<td className={`${cellClass} min-w-32 w-32`}>
                {formatDate(log.time)}
			</td>
            <td className={`${cellClass} min-w-32 w-32`}>
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
						<p className="font-normal">{log.level}</p>
					</div>
                </div>
			</td>
            <td className={`${cellClass} min-w-32 w-32`}>
                {log.message}
			</td>
		</>
	);

});