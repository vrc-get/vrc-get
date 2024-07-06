"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {FilePathRow} from "@/components/common-setting-parts";
import {environmentPickProjectDefaultPath} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {isWindows} from "@/lib/os";
import {BodyProps, SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={tc("setup:project-path:heading")}
		Body={Body}
		nextPage={"/setup/backups"}
		pageId={"ProjectPath"}
	/>
}

function Body(
	{
		environment,
		refetch,
	}: BodyProps
) {
	const projectPath = environment.default_project_path;

	const localAppDataPath = useQuery({
		queryKey: ["cacheDir"],
		queryFn: async () => await (await import("@tauri-apps/api/path")).cacheDir()
	}).data;

	const hasWhitespace = projectPath.includes(" ");
	const hasNonAscii = isWindows() && !projectPath.match(/[^\x00-\x7F]/);
	const inLocalAppData = !!(isWindows() && localAppDataPath && projectPath.includes(localAppDataPath));

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
			{hasWhitespace && <p className={"text-warning whitespace-normal text-sm"}>{tc("setup:project-path:warning:whitespace")}</p>}
			{hasNonAscii && <p className={"text-warning whitespace-normal text-sm"}>{tc("setup:project-path:warning:non-ascii")}</p>}
			{inLocalAppData && <p className={"text-warning whitespace-normal text-sm"}>{tc("setup:project-path:warning:in-local-app-data")}</p>}
		</>
	)
}
