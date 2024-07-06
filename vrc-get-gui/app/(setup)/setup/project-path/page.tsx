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
		heading={"Project Save Path"}
		Body={Body}
		nextPage={"/setup/backups"}
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
				When you crete project with ALCOM, the project will be saved in this path.<br/>
				This setting may also be changed in settings page later.
			</CardDescription>
			<FilePathRow
				withoutSelect
				path={projectPath}
				pick={environmentPickProjectDefaultPath}
				refetch={refetch}
				successMessage={tc("settings:toast:default project path updated")}
			/>
			{
				hasWhitespace
					? <p className={"text-warning whitespace-normal text-sm"}>
						The path contains whitespace. Whitespace in the path may cause problems with Unity and other tools.
					</p>
					: null
			}
			{
				hasNonAscii
					? <p className={"text-warning whitespace-normal text-sm"}>
						The path contains non-ASCII characters.
						Non-ASCII characters in the path with non-UTF-8 locale may cause problems with Unity and other tools.
					</p>
					: null
			}
			{
				inLocalAppData
					? <p className={"text-warning whitespace-normal text-sm"}>
						The location is in LocalAppData folder, which will be deleted with "Reset your PC" with "Keep my files".
						It's recommended to save your projects in a different location.
					</p>
					: null
			}
		</>
	)
}
