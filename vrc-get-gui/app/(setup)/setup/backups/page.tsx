"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {BackupFormatSelect, FilePathRow} from "@/components/common-setting-parts";
import {environmentPickProjectBackupPath, environmentSetBackupFormat} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {toastThrownError} from "@/lib/toast";
import {BodyProps, SetupPageBase} from "../setup-page-base";
import {useQuery} from "@tanstack/react-query";
import {isWindows, loadOSApi} from "@/lib/os";

export default function Page() {
	const osType = useQuery({
		queryKey: ["osType"],
		queryFn: async () => loadOSApi().then(os => os.type()),
		initialData: "Windows_NT" as const
	}).data;

	const isMac = osType === "Darwin";

	return <SetupPageBase
		heading={tc("setup:backups:heading")}
		Body={Body}
		nextPage={isMac ? "/setup/finish" : "/setup/system-setting"}
		prevPage={"/setup/project-path"}
		pageId={"Backups"}
	/>
}

function Body({environment, refetch}: BodyProps) {
	const projectBackupPath = environment.project_backup_path;
	const backupFormat = environment.backup_format;

	const setBackupFormat = async (format: string) => {
		try {
			await environmentSetBackupFormat(format)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const localAppDataPath = useQuery({
		queryKey: ["cacheDir"],
		queryFn: async () => await (await import("@tauri-apps/api/path")).cacheDir()
	}).data;

	const inLocalAppData = !!(isWindows() && localAppDataPath && projectBackupPath.includes(localAppDataPath));

	return (
		<>
			<h3>{tc("setup:backups:location")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:backups:location description")}
			</CardDescription>
			<FilePathRow
				withoutSelect
				path={projectBackupPath}
				pick={environmentPickProjectBackupPath}
				refetch={refetch}
				successMessage={tc("settings:toast:backup path updated")}
			/>
			{inLocalAppData && <p className={"text-warning whitespace-normal text-sm"}>{tc("setup:backups:warning:in-local-app-data")}</p>}
			<div className={"pb-3"}/>
			<h3>{tc("setup:backups:archive")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:backups:archive description")}
			</CardDescription>
			<BackupFormatSelect backupFormat={backupFormat} setBackupFormat={setBackupFormat}/>
		</>
	)
}
