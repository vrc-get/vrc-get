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
		heading={"Project Backup Settings"}
		Body={Body}
		nextPage={isMac ? "/setup/finish" : "/setup/system-setting"}
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
			<h3>{tc("settings:backup:path")}</h3>
			<CardDescription className={"whitespace-normal"}>
				When you backup your project, the backup archive file will be saved in this path.<br/>
				This setting may also be changed in settings page later.
			</CardDescription>
			<FilePathRow
				withoutSelect
				path={projectBackupPath}
				pick={environmentPickProjectBackupPath}
				refetch={refetch}
				successMessage={tc("settings:toast:backup path updated")}
			/>
			{
				inLocalAppData
					? <p className={"text-warning whitespace-normal text-sm"}>
						The location is in LocalAppData folder, which will be deleted with "Reset your PC" with "Keep my files".
						It's recommended to save your projects in a different location.
					</p>
					: null
			}
			<div className={"pb-3"}/>
			<h3>Archive Format</h3>
			<CardDescription className={"whitespace-normal"}>
				You can select the format of the backup archive. <br/>
				Default setting is the best format ALCOM thinks at the moment. <br/>
				Since the many big file in Unity project is compressed binary file,
				ALCOM currently thinks reducing backup size are not worth the compression time. <br/>
				However, if you have very limited disk space, it might be better to compress files.
			</CardDescription>
			<BackupFormatSelect backupFormat={backupFormat} setBackupFormat={setBackupFormat}/>
		</>
	)
}
