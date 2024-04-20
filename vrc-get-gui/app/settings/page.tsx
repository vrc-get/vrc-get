"use client"

import {Button, Card, Checkbox, Input, Typography} from "@material-tailwind/react";
import Link from "next/link";
import {useQuery} from "@tanstack/react-query";
import {
	environmentGetSettings,
	environmentPickProjectBackupPath,
	environmentPickProjectDefaultPath,
	environmentPickUnity,
	environmentPickUnityHub, environmentSetBackupFormat,
	environmentSetLanguage,
	environmentSetShowPrereleasePackages,
	TauriEnvironmentSettings,
	utilGetVersion,
} from "@/lib/bindings";
import {HNavBar, VStack} from "@/components/layout";
import React from "react";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import i18next, {languages, tc, tt} from "@/lib/i18n";
import {VGOption, VGSelect} from "@/components/select";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {emit} from "@tauri-apps/api/event";
import {type as osType, platform as osPlatform, version as osVersion, arch as osArch} from '@tauri-apps/api/os';

export default function Page() {
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	let body;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{tc("error loading settings")}</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>{tc("loading...")}</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch}/>;
			break;
		default:
			const _exhaustiveCheck: never = result;
	}

	return (
		<VStack className={"p-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("settings")}
				</Typography>
			</HNavBar>
			{body}
		</VStack>
	);
}

function Settings(
	{
		settings,
		refetch,
	}: {
		settings: TauriEnvironmentSettings,
		refetch: () => void
	}
) {
	const [pickUnity, unityDialog] = useFilePickerFunction(environmentPickUnity);
	const [pickUnityHub, unityHubDialog] = useFilePickerFunction(environmentPickUnityHub);
	const [pickProjectDefaultPath, projectDefaultDialog] = useFilePickerFunction(environmentPickProjectDefaultPath);
	const [pickProjectBackupPath, projectBackupDialog] = useFilePickerFunction(environmentPickProjectBackupPath);

	const selectUnityHub = async () => {
		try {
			const result = await pickUnityHub();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("selected file is invalid as a unity hub"));
					break;
				case "Successful":
					toastSuccess(tt("updated unity hub successfully!"));
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const addUnity = async () => {
		try {
			const result = await pickUnity();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("selected file is invalid as a unity"));
					break;
				case "AlreadyAdded":
					toastError(tt("selected unity is already added"));
					break;
				case "Successful":
					toastSuccess(tt("added unity successfully!"));
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const selectProjectDefaultFolder = async () => {
		try {
			const result = await pickProjectDefaultPath();
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("selected directory is invalid as the project default path"));
					break;
				case "Successful":
					toastSuccess(tt("updated the project default path successfully!"));
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	};

	const selectProjectBackupFolder = async () => {
		try {
			const result = await pickProjectBackupPath();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("selected directory is invalid as a project backup path"));
					break;
				case "Successful":
					toastSuccess(tt("updated the project backup path successfully!"));
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	};

	const setBackupFormat = async (format: string) => {
		try {
			await environmentSetBackupFormat(format)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const toggleShowPrereleasePackages = async (e: React.ChangeEvent<HTMLInputElement>) => {
		try {
			await environmentSetShowPrereleasePackages(e.target.checked)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const changeLanguage = async (value: string) => {
		await Promise.all([
			i18next.changeLanguage(value),
			environmentSetLanguage(value),
		])
	};

	const reportIssue = async () => {
		const url = new URL("https://github.com/vrc-get/vrc-get/issues/new")
		url.searchParams.append("labels", "bug,vrc-get-gui")
		url.searchParams.append("template", "01_gui_bug-report.yml")
		url.searchParams.append("os", `${await osType()} - ${await osPlatform()} - ${await osVersion()} - ${await osArch()}`)
		const appVersion = await utilGetVersion();
		url.searchParams.append("version", appVersion)
		url.searchParams.append("version", appVersion)

		const { open: shellOpen } = await import('@tauri-apps/api/shell');
		shellOpen(url.toString())
	}


	return (
		<main className="flex flex-col gap-2 flex-shrink overflow-y-auto flex-grow">
			<Card className={"flex-shrink-0 p-4"}>
				<h2 className={"pb-2"}>{tc("unity hub")}</h2>
				<div className={"flex gap-1"}>
					{
						settings.unity_hub
							? <Input className="flex-auto" value={settings.unity_hub} disabled/>
							: <Input value={"Unity Hub Not Found"} disabled className={"flex-auto text-red-900"}/>
					}
					<Button className={"flex-none px-4"} onClick={selectUnityHub}>{tc("select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<div className={"pb-2 flex align-middle"}>
					<div className={"flex-grow flex items-center"}>
						<h2>{tc("unity installations")}</h2>
					</div>
					<Button onClick={addUnity} size={"sm"} className={"m-1"}>{tc("add unity")}</Button>
				</div>
				<Card className="w-full overflow-x-auto overflow-y-scroll min-h-[20vh]">
					<UnityTable unityPaths={settings.unity_paths}/>
				</Card>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("default project path")}</h2>
				<Typography className={"whitespace-normal"}>
					{tc("the default project path is the directory where new projects are created in.")}
				</Typography>
				<div className={"flex gap-1"}>
					<Input className="flex-auto" value={settings.default_project_path} disabled/>
					<Button className={"flex-none px-4"} onClick={selectProjectDefaultFolder}>{tc("select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("backup")}</h2>
				<div className="mt-2">
					<h3>{tc("backup path")}</h3>
					<Typography className={"whitespace-normal"}>
						{tc("the backup path is the directory where alcom will create backup zips of the projects.")}
					</Typography>
					<div className={"flex gap-1"}>
						<Input className="flex-auto" value={settings.project_backup_path} disabled/>
						<Button className={"flex-none px-4"} onClick={selectProjectBackupFolder}>{tc("select")}</Button>
					</div>
				</div>
				<div className="mt-2">
					<label className={"flex items-center"}>
						<h3>{tc("backup archive format:")}</h3>
						<VGSelect value={tc("backup_format:" + settings.backup_format)} onChange={setBackupFormat}>
							<VGOption value={"default"}>{tc("backup_format:default")}</VGOption>
							<VGOption value={"zip-store"}>{tc("backup_format:zip-store")}</VGOption>
							<VGOption value={"zip-fast"}>{tc("backup_format:zip-fast")}</VGOption>
							<VGOption value={"zip-best"}>{tc("backup_format:zip-best")}</VGOption>
						</VGSelect>
					</label>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<Typography className={"whitespace-normal"}>
					{tc("description for show prerelease packages")}
				</Typography>
				<label className={"flex items-center"}>
					<Checkbox checked={settings.show_prerelease_packages} onChange={toggleShowPrereleasePackages}/>
					{tc("show prerelease packages")}
				</label>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<label className={"flex items-center"}>
					<h2>{tc("language")}: </h2>
					<VGSelect value={tc("langName")} onChange={changeLanguage} menuClassName={"w-96"}>
						{
							languages.map((lang) => (
								<VGOption key={lang} value={lang}>{tc("langName", {lng: lang})}</VGOption>
							))
						}
					</VGSelect>
				</label>
			</Card>
			{unityDialog}
			{unityHubDialog}
			{projectDefaultDialog}
			{projectBackupDialog}
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("check for updates")}</h2>
				<div>
					<Button onClick={() => emit("tauri://update")}>{tc("check for updates")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("report an issue")}</h2>
				<div>
				<Button onClick={reportIssue}>{tc("open an issue")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("licenses")}</h2>
				<Typography className={"whitespace-normal"}>
					{tc("click <l>here</l> to view licenses of the projects used in alcom", {}, {
						components: {l: <Link href={"/settings/licenses"} className={"underline"}/>}
					})}
				</Typography>
			</Card>
		</main>
	)
}

function UnityTable(
	{
		unityPaths,
	}: {
		unityPaths: [path: string, version: string, fromHub: boolean][]
	}
) {
	const UNITY_TABLE_HEAD = ["unity version", "unity path", "source"];
	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				{UNITY_TABLE_HEAD.map((head, index) => (
					<th key={index}
							className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
						<Typography variant="small" className="font-normal leading-none">{tc(head)}</Typography>
					</th>
				))}
			</tr>
			</thead>
			<tbody>
			{
				unityPaths.map(([path, version, isFromHub]) => (
					<tr key={path}>
						<td className={"p-2.5"}>{version}</td>
						<td className={"p-2.5"}>{path}</td>
						<td className={"p-2.5"}>
							{isFromHub ? tc("unity hub") : tc("manual")}
						</td>
					</tr>
				))
			}
			</tbody>
		</table>
	)
}
