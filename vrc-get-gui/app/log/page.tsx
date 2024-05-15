"use client";

import {Card, Typography} from "@material-tailwind/react";
import {HNavBar, VStack} from "@/components/layout";
import React, {useCallback, useEffect} from "react";
import {LogEntry, utilGetLogEntries} from "@/lib/bindings";
import {notoSansMono} from "@/app/fonts";
import {tc} from "@/lib/i18n";
import {useTauriListen} from "@/lib/use-tauri-listen";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);

	useEffect(() => {
		utilGetLogEntries().then(list => setLogEntries(list.toReversed()));
	}, []);

	useTauriListen<LogEntry>("log", useCallback((event) => {
		setLogEntries((entries) => {
			const entry = event.payload as LogEntry;
			return [entry, ...entries];
		});
	}, []));

	return (
		<VStack className={"m-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("logs")}
				</Typography>
			</HNavBar>
			<main className="flex-shrink overflow-hidden flex flex-grow">
				<Card
					className={`w-full overflow-x-auto overflow-y-scroll p-2 whitespace-pre ${notoSansMono.className} shadow-none`}>
					{logEntries.map((entry) => logEntryToText(entry)).join("\n")}
				</Card>
			</main>
		</VStack>
	);
}

function logEntryToText(entry: LogEntry) {
	return `${entry.time} [${entry.level.padStart(5, ' ')}] ${entry.target}: ${entry.message}`;
}

