"use client";

import {Card, CardDescription, CardFooter, CardHeader} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {BackupFormatSelect, FilePathRow} from "@/components/common-setting-parts";
import {useRouter} from "next/navigation";
import {environmentGetSettings, environmentPickProjectBackupPath, environmentSetBackupFormat} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {toastThrownError} from "@/lib/toast";

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
		router.push("/setup/system-setting")
	};

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>Project Backup Settings</h1>
			</CardHeader>
			<div className={"pb-4"}/>
			{
				!result.data
					? <p>Loading...</p>
					: <WithLoadedData
						projectBackupPath={result.data.project_backup_path}
						backupFormat={result.data.backup_format}
						refetch={() => result.refetch()}
					/>
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
		projectBackupPath,
		backupFormat,
		refetch,
	}: {
		projectBackupPath: string;
		backupFormat: string;
		refetch: () => void;
	}
) {
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
