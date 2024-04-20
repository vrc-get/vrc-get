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
import {HNavBar, VStack, HContent, HSection, HSectionTitle, HSectionRow, HSectionSubTitle, HSectionText} from "@/components/layout";
import React from "react";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import i18next, {languages, tc, tt} from "@/lib/i18n";
import {VGOption, VGSelect} from "@/components/select";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {emit} from "@tauri-apps/api/event";
import {shellOpen} from "@/lib/shellOpen";
import { loadOSApi } from "@/lib/os";
import Table from "@/components/Table";

export default function Page() {
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	let body;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{tc("settings:error:load error")}</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>{tc("general:loading...")}</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch} />
			break;
		default:
			const _exhaustiveCheck: never = result;
	}

	return (
		<VStack className={"p-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography variant="h4" className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("settings")}
				</Typography>
			</HNavBar>
			<HContent>
				{body}
			</HContent>
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
					toastError(tt("settings:toast:not unity hub"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:unity hub path updated"));
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
					toastError(tt("settings:toast:not unity"));
					break;
				case "AlreadyAdded":
					toastError(tt("settings:toast:unity already added"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:unity added"));
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
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:default project path updated"));
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
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:backup path updated"));
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
		const osApi = await loadOSApi();
		url.searchParams.append("os", `${await osApi.type()} - ${await osApi.platform()} - ${await osApi.version()} - ${await osApi.arch()}`)
		const appVersion = await utilGetVersion();
		url.searchParams.append("version", appVersion)
		url.searchParams.append("version", appVersion)

		shellOpen(url.toString())
	}


	return (
		<>
			<HSection>
				<HSectionTitle>{tc("unity hub")}</HSectionTitle>
				<HSectionRow>
						{
							settings.unity_hub
								? <Input className="flex-auto" value={settings.unity_hub} disabled/>
								: <Input value={"Unity Hub Not Found"} disabled className={"flex-auto text-red-900"}/>
						}
					<Button className={"flex-none px-4"} onClick={selectUnityHub}>{tc("select")}</Button>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionRow>
					<HSectionTitle className="flex-grow">{tc("unity installations")}</HSectionTitle>
					<Button onClick={addUnity} size={"sm"} className={"m-1"}>{tc("add unity")}</Button>
				</HSectionRow>
				<HSectionRow>
					<Table
						className="w-full min-h-[20vh] max-h-[30vh] overflow-y-auto"
						layout={["auto", "2fr", "auto"]}
						header={[tc("settings:unity:version"), tc("settings:unity:path"), tc("general:source")]}
						rows={settings.unity_paths.map((path) => ([
							path[1],
							path[0],
							path[2] ? tc("settings:unity hub") : tc("settings:unity:source:manual")
						]))}
					/>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("default project path")}
				</HSectionTitle>
				<HSectionSubTitle>
					{tc("the default project path is the directory where new projects are created in.")}
				</HSectionSubTitle>
				<HSectionRow>
					<Input className="flex-auto" value={settings.default_project_path} disabled/>
					<Button className={"flex-none px-4"} onClick={selectProjectDefaultFolder}>{tc("select")}</Button>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("backup")}
				</HSectionTitle>
				<HSectionSubTitle>
					{tc("backup path")}
				</HSectionSubTitle>
				<HSectionText>
					{tc("the backup path is the directory where alcom will create backup zips of the projects.")}
				</HSectionText>
				<HSectionRow>
					<Input className="flex-auto" value={settings.project_backup_path} disabled/>
					<Button className={"flex-none px-4"} onClick={selectProjectBackupFolder}>{tc("select")}</Button>
				</HSectionRow>
				<HSectionRow>
					<HSectionSubTitle>
						{tc("Backup archive format")} :
					</HSectionSubTitle>
					<VGSelect value={tc("backup_format:" + settings.backup_format)} onChange={setBackupFormat}>
						<VGOption value={"default"}>{tc("backup_format:default")}</VGOption>
						<VGOption value={"zip-store"}>{tc("backup_format:zip-store")}</VGOption>
						<VGOption value={"zip-fast"}>{tc("backup_format:zip-fast")}</VGOption>
						<VGOption value={"zip-best"}>{tc("backup_format:zip-best")}</VGOption>
					</VGSelect>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("Prerelease packages")}
				</HSectionTitle>
				<HSectionText>
					{tc("description for show prerelease packages")}
				</HSectionText>
				<HSectionRow>
					<Checkbox checked={settings.show_prerelease_packages} onChange={toggleShowPrereleasePackages}/>
					{tc("show prerelease packages")}
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("settings:language")}
				</HSectionTitle>
				<HSectionRow>
					<VGSelect value={tc("settings:langName")} onChange={changeLanguage} menuClassName={"w-96"}>
						{
							languages.map((lang) => (
								<VGOption key={lang} value={lang}>{tc("settings:langName", {lng: lang})}</VGOption>
							))
						}
					</VGSelect>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("check for updates")}
				</HSectionTitle>
				<HSectionRow>
					<Button onClick={() => emit("tauri://update")}>{tc("check for updates")}</Button>
				</HSectionRow>
			</HSection>
			<HSection className={"flex-shrink-0 p-4"}>
				<HSectionTitle>
					{tc("settings:report issue")}
				</HSectionTitle>
				<HSectionRow>
					<Button onClick={reportIssue}>{tc("settings:button:open issue")}</Button>
				</HSectionRow>
			</HSection>
			<HSection>
				<HSectionTitle>
					{tc("licenses")}
				</HSectionTitle>
				<HSectionText>
					{tc("click <l>here</l> to view licenses of the projects used in alcom", {}, {
						components: {l: <Link href={"/settings/licenses"} className={"underline"}/>}
					})}
				</HSectionText>
			</HSection>
			{unityDialog}
			{unityHubDialog}
			{projectDefaultDialog}
			{projectBackupDialog}
		</>
	)
}
