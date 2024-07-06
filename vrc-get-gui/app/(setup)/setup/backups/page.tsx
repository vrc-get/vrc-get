"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {BackupFormatSelect, FilePathRow} from "@/components/common-setting-parts";
import {environmentPickProjectBackupPath, environmentSetBackupFormat} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {toastThrownError} from "@/lib/toast";
import {BodyProps, SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={"Project Backup Settings"}
		Body={Body}
		nextPage={"/setup/system-setting"}
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
