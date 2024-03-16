"use client"

import {Button, Card, Checkbox, Input, Typography} from "@material-tailwind/react";
import Link from "next/link";
import {useQuery} from "@tanstack/react-query";
import {
	environmentGetSettings,
	environmentPickProjectBackupPath,
	environmentPickProjectDefaultPath,
	environmentPickUnity,
	environmentPickUnityHub,
	environmentSetShowPrereleasePackages,
	TauriEnvironmentSettings
} from "@/lib/bindings";
import {VStack} from "@/components/layout";
import React from "react";
import {Trans, useTranslation} from "react-i18next";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";

export default function Page() {
	const {t} = useTranslation();
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	let body;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{t("error loading settings")}</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>{t("loading...")}</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch}/>;
			break;
		default:
			const _exhaustiveCheck: never = result;
	}

	return (
		<VStack className={"p-4 overflow-y-auto"}>
			{body}
			<Card className={"flex-shrink-0 p-4"}>
				<h2>Licenses</h2>
				<Typography className={"whitespace-normal"}>
					<Trans
						i18nKey={"clock <l>here</l> to view licenses of the projects used in vrc-get-gui"}
						components={{l: <Link href={"/settings/licenses"} className={"underline"}/>}}
					/>
				</Typography>
			</Card>
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
	const {t} = useTranslation();

	const selectUnityHub = async () => {
		try {
			const result = await environmentPickUnityHub();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(t("selected file is invalid as a unity hub"));
					break;
				case "Successful":
					toastSuccess(t("updated unity hub successfully!"));
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
			const result = await environmentPickUnity();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(t("selected file is invalid as a unity"));
					break;
				case "AlreadyAdded":
					toastError(t("selected unity is already added"));
					break;
				case "Successful":
					toastSuccess(t("added unity successfully!"));
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
			const result = await environmentPickProjectDefaultPath();
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(t("selected directory is invalid as the project default path"));
					break;
				case "Successful":
					toastSuccess(t("updated the project default path successfully!"));
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
			const result = await environmentPickProjectBackupPath();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(t("selected directory is invalid as a project backup path"));
					break;
				case "Successful":
					toastSuccess(t("updated the project backup path successfully!"));
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

	const toggleShowPrereleasePackages = async (e: React.ChangeEvent<HTMLInputElement>) => {
		try {
			await environmentSetShowPrereleasePackages(e.target.checked)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	return (
		<>
			<Card className={"flex-shrink-0 p-4"}>
				<h1>{t("settings")}</h1>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2 className={"pb-2"}>{t("unity hub")}</h2>
				<div className={"flex gap-1"}>
					{
						settings.unity_hub
							? <Input value={settings.unity_hub} disabled/>
							: <Input value={"Unity Hub Not Found"} disabled className={"text-red-900"}/>
					}
					<Button className={"px-4"} onClick={selectUnityHub}>{t("select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<div className={"pb-2 flex align-middle"}>
					<div className={"flex-grow flex items-center"}>
						<h2>{t("unity installations")}</h2>
					</div>
					<Button onClick={addUnity} size={"sm"} className={"m-1"}>{t("add unity")}</Button>
				</div>
				<Card className="w-full overflow-x-auto overflow-y-scroll min-h-[20vh]">
					<UnityTable unityPaths={settings.unity_paths}/>
				</Card>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{t("default project path")}</h2>
				<Typography className={"whitespace-normal"}>
					{t("the default project path is the directory where new projects are created in.")}
				</Typography>
				<div className={"flex gap-1"}>
					<Input value={settings.default_project_path} disabled/>
					<Button className={"px-4"} onClick={selectProjectDefaultFolder}>{t("select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{t("backup path")}</h2>
				<Typography className={"whitespace-normal"}>
					{t("the backup path is the directory where vrc-get-gui will create backup zips of the projects.")}
				</Typography>
				<div className={"flex gap-1"}>
					<Input value={settings.project_backup_path} disabled/>
					<Button className={"px-4"} onClick={selectProjectBackupFolder}>{t("select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<Typography className={"whitespace-normal"}>
					{t("description for show prerelease packages")}
				</Typography>
				<label className={"flex items-center"}>
					<Checkbox checked={settings.show_prerelease_packages} onChange={toggleShowPrereleasePackages}/>
					{t("show prerelease packages")}
				</label>
			</Card>
		</>
	)
}

function UnityTable(
	{
		unityPaths,
	}: {
		unityPaths: [path: string, version: string, fromHub: boolean][]
	}
) {
	const {t} = useTranslation();
	const UNITY_TABLE_HEAD = ["unity version", "unity path", "source"];
	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				{UNITY_TABLE_HEAD.map((head, index) => (
					<th key={index}
							className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
						<Typography variant="small" className="font-normal leading-none">{t(head)}</Typography>
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
							{isFromHub ? t("unity hub") : t("manual")}
						</td>
					</tr>
				))
			}
			</tbody>
		</table>
	)
}
