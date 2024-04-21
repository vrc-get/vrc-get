"use client";

import {Card, Typography} from "@material-tailwind/react";
import {HContent, HNavBar, HSection, HSectionText, VStack} from "@/components/layout";
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
				<Typography variant="h4" className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("logs")}
				</Typography>
			</HNavBar>
			<HContent>
				<HSection className="overflow-x-auto">
					{logEntries.map((entry, i) => (
						<HSectionText className="text-nowrap" key={i}>
							{logEntryToText(entry)}
						</HSectionText>
					))}
				</HSection>
			</HContent>
			{/* <main className="flex-shrink overflow-hidden flex flex-grow">
				<Card className={`w-full overflow-x-auto overflow-y-scroll p-2 whitespace-pre ${notoSansMono.className} shadow-none`}>
				{logEntries.map((entry) => logEntryToText(entry)).join("\n")}
				</Card>
			</main> */}
		</VStack>
	);
}

function logEntryToText(entry: LogEntry) {
	const level = entry.level
	let levelClassName;
	switch(level){
		case "Debug":
			levelClassName = "bg-blue-600"
			break
		case "Info":
			levelClassName = "bg-green-600"
			break
		case "Warn":
			levelClassName = "bg-orange-600"
			break
		case "Error":
			levelClassName = "bg-red-600"
			break
		default:
			levelClassName = "bg-inherit"
	}

	// Using a blank character to get space within a span component
	return (<><span className={`mr-1 ${levelClassName}`}>⠀</span>{entry.time} [{level}] {entry.target}: {entry.message}</>)
}

