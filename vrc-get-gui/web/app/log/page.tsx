"use client";

import {Card, Typography} from "@material-tailwind/react";
import {HNavBar, VStack} from "@/components/layout";
import React, {useEffect} from "react";
import {LogEntry, utilGetLogEntries} from "@/lib/bindings";
import {notoSansMono} from "@/app/fonts";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);

	useEffect(() => {
		utilGetLogEntries().then(setLogEntries);
	}, []);
	// time: string; level: LogLevel; target: string; message: string

	return (
		<VStack className={"m-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					Logs
				</Typography>
			</HNavBar>
			<main className="flex-shrink overflow-hidden flex flex-grow">
				<Card className={`w-full overflow-x-auto overflow-y-scroll p-2 whitespace-pre ${notoSansMono.className}`}>
					{logEntries.map((entry) => logEntryToText(entry)).join("\n")}
				</Card>
			</main>
		</VStack>
	);
}

function logEntryToText(entry: LogEntry) {
	return `${entry.time} [${entry.level.padStart(5, ' ')}] ${entry.target}: ${entry.message}`;
}

