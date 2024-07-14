"use client";

import { CardDescription } from "@/components/ui/card";
import React from "react";
import { FilePathRow } from "@/components/common-setting-parts";
import { environmentPickProjectDefaultPath } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import {
	type BodyProps,
	SetupPageBase,
	WarningMessage,
} from "../setup-page-base";
import { useGlobalInfo } from "@/lib/global-info";

export default function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:project-path:heading")}
			Body={Body}
			nextPage={"/setup/backups"}
			prevPage={"/setup/unity-hub"}
			pageId={"ProjectPath"}
		/>
	);
}

function Body({ environment, refetch }: BodyProps) {
	const projectPath = environment.default_project_path;

	const hasWhitespace = projectPath.includes(" ");
	const globalInfo = useGlobalInfo();
	const isWindows = globalInfo.osType === "WindowsNT";
	// biome-ignore lint/suspicious/noControlCharactersInRegex: allow control characters
	const hasNonAscii = isWindows && projectPath.match(/[^\x00-\x7F]/);
	const inLocalAppData = !!(
		isWindows &&
		globalInfo.localAppData &&
		projectPath.includes(globalInfo.localAppData)
	);

	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:project-path:description")}
			</CardDescription>
			<FilePathRow
				withoutSelect
				path={projectPath}
				pick={environmentPickProjectDefaultPath}
				refetch={refetch}
				successMessage={tc("settings:toast:default project path updated")}
			/>
			<div className="flex flex-col gap-1">
				{hasWhitespace && (
					<WarningMessage>
						{tc("setup:project-path:warning:whitespace")}
					</WarningMessage>
				)}
				{hasNonAscii && (
					<WarningMessage>
						{tc("setup:project-path:warning:non-ascii")}
					</WarningMessage>
				)}
				{inLocalAppData && (
					<WarningMessage>
						{tc("setup:project-path:warning:in-local-app-data")}
					</WarningMessage>
				)}
			</div>
		</>
	);
}
