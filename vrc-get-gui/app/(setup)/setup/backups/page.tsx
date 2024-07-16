"use client";

import {
	BackupFormatSelect,
	FilePathRow,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import {
	environmentPickProjectBackupPath,
	environmentSetBackupFormat,
} from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import {
	type BodyProps,
	SetupPageBase,
	WarningMessage,
} from "../setup-page-base";

export default function Page() {
	const isMac = useGlobalInfo().osType === "Darwin";

	return (
		<SetupPageBase
			heading={tc("setup:backups:heading")}
			Body={Body}
			nextPage={isMac ? "/setup/finish" : "/setup/system-setting"}
			prevPage={"/setup/project-path"}
			pageId={"Backups"}
		/>
	);
}

function Body({ environment, refetch }: BodyProps) {
	const projectBackupPath = environment.project_backup_path;
	const backupFormat = environment.backup_format;

	const setBackupFormat = async (format: string) => {
		try {
			await environmentSetBackupFormat(format);
			refetch();
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const globalInfo = useGlobalInfo();
	const isWindows = globalInfo.osType === "WindowsNT";
	const inLocalAppData = !!(
		isWindows &&
		globalInfo.localAppData &&
		projectBackupPath.includes(globalInfo.localAppData)
	);

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
			{inLocalAppData && (
				<WarningMessage>
					{tc("setup:backups:warning:in-local-app-data")}
				</WarningMessage>
			)}
			<div className={"pb-3"} />
			<h3>{tc("setup:backups:archive")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:backups:archive description")}
			</CardDescription>
			<BackupFormatSelect
				backupFormat={backupFormat}
				setBackupFormat={setBackupFormat}
			/>
		</>
	);
}
