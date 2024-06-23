"use client";

import {Card} from "@/components/ui/card";
import {HNavBar, VStack} from "@/components/layout";
import React, {useCallback, useEffect} from "react";
import {LogEntry, utilGetLogEntries} from "@/lib/bindings";
import {notoSansMono} from "@/app/fonts";
import {tc} from "@/lib/i18n";
import {useTauriListen} from "@/lib/use-tauri-listen";
import {ScrollArea, ScrollBar} from "@/components/ui/scroll-area";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);

	useEffect(() => {
		utilGetLogEntries().then(list => setLogEntries([...list].reverse()));
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
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("logs")}
				</p>
			</HNavBar>
			<Card className={`w-full p-2 whitespace-pre font-mono shadow-none text-muted-foreground overflow-hidden`}>
				<ScrollArea type={"always"} className={"w-full h-full"}>
						{logEntries.map((entry) => logEntryToText(entry)).join("\n")}
					<ScrollBar className={"bg-background"} orientation="horizontal" />
				</ScrollArea>
			</Card>
		</VStack>
	);
}

function logEntryToText(entry: LogEntry) {
	return `${entry.time} [${entry.level.padStart(5, ' ')}] ${entry.target}: ${entry.message}`;
}
