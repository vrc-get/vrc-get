"use client";

import { HNavBar, VStack } from "@/components/layout";
import type { LogEntry } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { useTauriListen } from "@/lib/use-tauri-listen";
import React, { useCallback, useEffect } from "react";
import { LogListCard } from "./log-list-card";

export default function Page() {
	const [logEntries, setLogEntries] = React.useState<LogEntry[]>([]);
	const globalInfo = useGlobalInfo();

	useEffect(() => {
		commands
			.utilGetLogEntries()
			.then((list) => setLogEntries([...list].reverse()));
	}, []);

	useTauriListen<LogEntry>(
		"log",
		useCallback((event) => {
			setLogEntries((entries) => {
				const entry = event.payload as LogEntry;
				return [entry, ...entries];
			});
		}, []),
	);

	return (
		<VStack>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("logs")}
				</p>
				<div className={"flex-grow"} />
			</HNavBar>
			<main className="flex-shrink overflow-hidden flex w-full">
				<LogListCard logEntry={logEntries.reverse()}>
				</LogListCard>
			</main>
		</VStack>
	);
}

function logEntryToText(entry: LogEntry) {
	return `${entry.time} [${entry.level.padStart(5, " ")}] ${entry.target}: ${entry.message}`;
}
