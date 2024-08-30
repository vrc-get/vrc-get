import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { DropdownMenu, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { LogEntry } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { memo, useState } from "react";

export const LogListCard = memo(function LogListCard({
    LogEntry
}: {
    LogEntry: LogEntry[]
}) {
    const [search, setSearch] = useState("");

    const TABLE_HEAD = [
		"logs:time",
		"logs:log level",
		"logs:detail",
	];

    return (
		<Card className="flex-grow flex-shrink flex shadow-none w-full">
            <CardContent className="w-full p-2 flex flex-col gap-2">
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
						{LogEntry.map((row) => (
							<tr>
                                <LogRow log={{
                                    time: `${row.time}`,
                                    level: row.level,
                                    target: row.target,
                                    message: row.message,
                                    gui_toast: row.gui_toast
                                }}>
                                    
                                </LogRow>
							</tr>
						))}
					</tbody>
				</ScrollableCardTable>
			</CardContent>
		</Card>
	);

});

function ManageLogsHeading({
	onRefresh,
	search,
	setSearch,
}: {
	onRefresh: () => void;
	search: string;
	setSearch: (value: string) => void;
}) {
    return (
		<div
			className={
				"flex flex-wrap flex-shrink-0 flex-grow-0 flex-row gap-2 items-center"
			}
		>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
				{tc("projects:manage:manage packages")}
			</p>

			<SearchBox
				className={"w-max flex-grow"}
				value={search}
				onChange={(e) => setSearch(e.target.value)}
			/>

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button className={"flex-shrink-0 p-3"}>
						{tc("projects:manage:button:select repositories")}
					</Button>
				</DropdownMenuTrigger>
			</DropdownMenu>
		</div>
	);

}

const LogRow = memo(function LogRow({
	log,
}: {
	log: LogEntry;
}) {
    const cellClass = "p-2.5";
    const noGrowCellClass = `${cellClass} w-1`;


    return (
		<>
			<td className={`${cellClass} min-w-32 w-32`}>
                ${log.time}
			</td>
            <td className={`${cellClass} min-w-32 w-32`}>
                ${log.level}
			</td>
            <td className={`${cellClass} min-w-32 w-32`}>
                ${log.message}
			</td>
		</>
	);

});