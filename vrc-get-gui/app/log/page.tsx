"use client";

import {Card, Typography} from "@material-tailwind/react";
import {HNavBar, VStack} from "@/components/layout";
import React, {useEffect} from "react";
import {LogEntry, utilGetLogEntries} from "@/lib/bindings";
import {notoSansMono} from "@/app/fonts";
import {listen} from '@tauri-apps/api/event';
import {tc} from "@/lib/i18n";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);

	useEffect(() => {
		utilGetLogEntries().then(list => setLogEntries(list.toReversed()));
	}, []);

	useEffect(() => {
		let unlisten: (() => void) | undefined = undefined;
		let unlistened = false;

		listen("log", (event) => {
			setLogEntries((entries) => {
				const entry = event.payload as LogEntry;
				return [entry, ...entries];
			});
		}).then((unlistenFn) => {
			if (unlistened) {
				unlistenFn();
			} else {
				unlisten = unlistenFn;
			}
		});

		return () => {
			unlisten?.();
			unlistened = true;
		};
	}, []);

	return (
		<VStack className={"m-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("logs")}
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

