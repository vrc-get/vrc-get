"use client";

import {Card, CardDescription, CardFooter, CardHeader} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {FilePathRow} from "@/components/common-setting-parts";
import {useRouter} from "next/navigation";
import {environmentGetSettings, environmentPickProjectDefaultPath} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {isWindows} from "@/lib/os";

export default function Page() {
	const router = useRouter();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	const onBack = () => {
		router.back()
	};

	const onNext = () => {
		// TODO: fetch next page from backend
	};

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>Project Save Path</h1>
			</CardHeader>
			<CardDescription className={"whitespace-normal"}>
				When you crete project with ALCOM, the project will be saved in this path.<br/>
				This setting may also be changed in settings page later.
			</CardDescription>
			<div className={"pb-4"}/>
			{
				!result.data
					? <p>Loading...</p>
					: <WithLoadedData projectPath={result.data.default_project_path} refetch={() => result.refetch()}/>
			}
			<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
				<Button onClick={onBack}>Back</Button>
				<Button onClick={onNext}>Next</Button>
			</CardFooter>
		</Card>
	</div>
}

function WithLoadedData(
	{
		projectPath,
		refetch,
	}: {
		projectPath: string;
		refetch: () => void;
	}
) {
	const localAppDataPath = useQuery({
		queryKey: ["cacheDir"],
		queryFn: async () => await (await import("@tauri-apps/api/path")).cacheDir()
	}).data;

	const hasWhitespace = projectPath.includes(" ");
	const hasNonAscii = isWindows() && !projectPath.match(/[^\x00-\x7F]/);
	const inLocalAppData = !!(isWindows() && localAppDataPath && projectPath.includes(localAppDataPath));

	return (
		<>
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
