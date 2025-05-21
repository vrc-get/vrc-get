"use client";

import {
	BackupFormatSelect,
	BackupPathWarnings,
	FilePathRow,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { type BodyProps, SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/backups/")({
	component: Page,
});

function Page() {
	const shouldInstallDeepLink = useGlobalInfo().shouldInstallDeepLink;

	return (
		<SetupPageBase
			heading={tc("setup:backups:heading")}
			Body={Body}
			nextPage={
				shouldInstallDeepLink ? "/setup/system-setting" : "/setup/finish"
			}
			prevPage={"/setup/project-path"}
			pageId={"Backups"}
		/>
	);
}

function Body({ environment }: BodyProps) {
	const projectBackupPath = environment.project_backup_path;
	const backupFormat = environment.backup_format;

	const queryClient = useQueryClient();

	const pickProjectBackupPath = useMutation({
		mutationFn: async () => await commands.environmentPickProjectBackupPath(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: (result) => {
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tc("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tc("settings:toast:backup path updated"));
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["environmentGetSettings"],
			});
		},
	});

	return (
		<>
			<h3>{tc("setup:backups:location")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:backups:location description")}
			</CardDescription>
			<FilePathRow
				path={projectBackupPath}
				pick={pickProjectBackupPath.mutate}
				withOpen={false}
			/>
			<BackupPathWarnings backupPath={projectBackupPath} />
			<div className={"pb-3"} />
			<h3>{tc("setup:backups:archive")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("settings:backup:format description")}
			</CardDescription>
			<BackupFormatSelect backupFormat={backupFormat} />
		</>
	);
}
