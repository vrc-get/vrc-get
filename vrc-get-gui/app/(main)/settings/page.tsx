"use client"

import {Button} from "@/components/ui/button";
import {Card} from "@/components/ui/card";
import {Checkbox} from "@/components/ui/checkbox";
import Link from "next/link";
import {useQuery} from "@tanstack/react-query";
import {
	CheckForUpdateResponse,
	deepLinkInstallVcc,
	environmentClearPackageCache,
	environmentGetSettings,
	environmentPickProjectBackupPath,
	environmentPickProjectDefaultPath,
	environmentPickUnity,
	environmentPickUnityHub,
	environmentSetBackupFormat,
	environmentSetReleaseChannel,
	environmentSetShowPrereleasePackages,
	environmentSetUseAlcomForVccProtocol,
	TauriEnvironmentSettings,
	utilCheckForUpdate,
	utilOpenUrl,
} from "@/lib/bindings";
import {HNavBar, VStack} from "@/components/layout";
import React, {useState} from "react";
import {toastError, toastNormal, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {ScrollableCardTable} from "@/components/ScrollableCardTable";
import {assertNever} from "@/lib/assert-never";
import {ScrollPageContainer} from "@/components/ScrollPageContainer";
import {CheckForUpdateMessage} from "@/components/CheckForUpdateMessage";
import {BackupFormatSelect, FilePathRow, LanguageSelector, ThemeSelector} from "@/components/common-setting-parts";
import globalInfo, {useGlobalInfo} from "@/lib/global-info";

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
			body = <Settings settings={result.data} refetch={result.refetch}/>;
			break;
		default:
			assertNever(result);
	}

	return (
		<VStack>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("settings")}
				</p>
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
	const isMac = useGlobalInfo().osType == "Darwin";

	return (
		<ScrollPageContainer>
			<main className="flex flex-col gap-2 flex-shrink flex-grow">
				<Card className={"flex-shrink-0 p-4"}>
					<h2 className={"pb-2"}>{tc("settings:unity hub path")}</h2>
					<FilePathRow
						withoutSelect
						path={settings.unity_hub}
						pick={environmentPickUnityHub}
						refetch={refetch}
						notFoundMessage={"Unity Hub Not Found"}
						successMessage={tc("settings:toast:unity hub path updated")}
					/>
				</Card>
				<UnityInstallationsCard refetch={refetch} unityPaths={settings.unity_paths}/>
				<Card className={"flex-shrink-0 p-4"}>
					<h2>{tc("settings:default project path")}</h2>
					<p className={"whitespace-normal"}>
						{tc("settings:default project path description")}
					</p>
					<FilePathRow
						path={settings.default_project_path}
						pick={environmentPickProjectDefaultPath}
						refetch={refetch}
						successMessage={tc("settings:toast:default project path updated")}
					/>
				</Card>
				<BackupCard
					projectBackupPath={settings.project_backup_path}
					backupFormat={settings.backup_format}
					refetch={refetch}
				/>
				<PackagesCard showPrereleasePackages={settings.show_prerelease_packages} refetch={refetch}/>
				<AppearanceCard/>
				<AlcomCard
					isMac={isMac}
					releaseChannel={settings.release_channel}
					useAlcomForVccProtocol={settings.use_alcom_for_vcc_protocol}
					refetch={refetch}
				/>
			</main>
		</ScrollPageContainer>
	)
}

function UnityInstallationsCard(
	{
		refetch,
		unityPaths,
	}: {
		refetch: () => void;
		unityPaths: [path: string, version: string, fromHub: boolean][]
	}
) {
	const [pickUnity, unityDialog] = useFilePickerFunction(environmentPickUnity);

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
					assertNever(result);
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const UNITY_TABLE_HEAD = ["settings:unity:version", "settings:unity:path", "general:source"];

	return (
		<Card className={"flex-shrink-0 p-4"}>
			<div className={"pb-2 flex align-middle"}>
				<div className={"flex-grow flex items-center"}>
					<h2>{tc("settings:unity installations")}</h2>
				</div>
				<Button onClick={addUnity} size={"sm"} className={"m-1"}>{tc("settings:button:add unity")}</Button>
			</div>
			<ScrollableCardTable className="w-full min-h-[20vh]">
				<thead>
				<tr>
					{UNITY_TABLE_HEAD.map((head, index) => (
						<th key={index}
								className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5`}>
							<small className="font-normal leading-none">{tc(head)}</small>
						</th>
					))}
				</tr>
				</thead>
				<tbody>
				{
					unityPaths.map(([path, version, isFromHub]) => (
						<tr key={path} className="even:bg-secondary/30">
							<td className={"p-2.5"}>{version}</td>
							<td className={"p-2.5"}>{path}</td>
							<td className={"p-2.5"}>
								{isFromHub ? tc("settings:unity:source:unity hub") : tc("settings:unity:source:manual")}
							</td>
						</tr>
					))
				}
				</tbody>
			</ScrollableCardTable>
			{unityDialog}
		</Card>
	)
}

function BackupCard(
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
		<Card className={"flex-shrink-0 p-4"}>
			<h2>{tc("projects:backup")}</h2>
			<div className="mt-2">
				<h3>{tc("settings:backup:path")}</h3>
				<p className={"whitespace-normal"}>
					{tc("settings:backup:path description")}
				</p>
				<FilePathRow
					path={projectBackupPath}
					pick={environmentPickProjectBackupPath}
					refetch={refetch}
					successMessage={tc("settings:toast:backup path updated")}
				/>
			</div>
			<div className="mt-2">
				<label className={"flex items-center"}>
					<h3>{tc("settings:backup:format")}</h3>
					<BackupFormatSelect backupFormat={backupFormat} setBackupFormat={setBackupFormat}/>
				</label>
			</div>
		</Card>
	)
}

function PackagesCard(
	{
		showPrereleasePackages,
		refetch,
	}: {
		showPrereleasePackages: boolean;
		refetch: () => void;
	}
) {
	const clearPackageCache = async () => {
		try {
			await environmentClearPackageCache()
			toastSuccess(tc("settings:toast:package cache cleared"))
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const toggleShowPrereleasePackages = async (e: "indeterminate" | boolean) => {
		try {
			await environmentSetShowPrereleasePackages(e === true)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	return (
		<Card className={"flex-shrink-0 p-4 flex flex-col gap-4"}>
			<h2>{tc("settings:packages")}</h2>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button onClick={clearPackageCache}>{tc("settings:clear package cache")}</Button>
			</div>
			<div>
				<label className={"flex items-center gap-2"}>
					<Checkbox checked={showPrereleasePackages} onCheckedChange={(e) => toggleShowPrereleasePackages(e)}/>
					{tc("settings:show prerelease")}
				</label>
				<p className={"text-sm whitespace-normal"}>{tc("settings:show prerelease description")}</p>
			</div>
		</Card>
	)
}

function AppearanceCard() {
	return (
		<Card className={"flex-shrink-0 p-4"}>
			<h2>{tc("settings:appearance")}</h2>
			<LanguageSelector/>
			<ThemeSelector/>
		</Card>
	)
}

function AlcomCard(
	{
		isMac,
		releaseChannel,
		useAlcomForVccProtocol,
		refetch,
	}: {
		isMac: boolean;
		releaseChannel: string;
		useAlcomForVccProtocol: boolean;
		refetch: () => void;
	}
) {
	const [updateState, setUpdateState] = useState<CheckForUpdateResponse | null>(null);

	const checkForUpdate = async () => {
		try {
			const checkVersion = await utilCheckForUpdate();
			if (checkVersion.is_update_available) {
				setUpdateState(checkVersion);
			} else {
				toastNormal(tc("check update:toast:no updates"));
			}
		} catch (e) {
			toastThrownError(e)
			console.error(e)
		}
	}

	const reportIssue = async () => {
		const url = new URL("https://github.com/vrc-get/vrc-get/issues/new")
		url.searchParams.append("labels", "bug,vrc-get-gui")
		url.searchParams.append("template", "01_gui_bug-report.yml")
		url.searchParams.append("os", `${globalInfo.osInfo} - ${globalInfo.arch}`)
		url.searchParams.append("version", globalInfo.version ?? "unknown")

		void utilOpenUrl(url.toString())
	}

	const changeReleaseChannel = async (value: "indeterminate" | boolean) => {
		await environmentSetReleaseChannel(value === true ? "beta" : "stable");
		refetch();
	};

	const changeUseAlcomForVcc = async (value: "indeterminate" | boolean) => {
		await environmentSetUseAlcomForVccProtocol(value === true);
		refetch();
	};

	const installVccProtocol = async () => {
		try {
			await deepLinkInstallVcc();
			toastSuccess(tc("settings:toast:vcc scheme installed"));
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	return (
		<Card className={"flex-shrink-0 p-4 flex flex-col gap-4"}>
			{updateState && <CheckForUpdateMessage response={updateState} close={() => setUpdateState(null)}/>}
			<h2>ALCOM</h2>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button onClick={checkForUpdate}>{tc("settings:check update")}</Button>
				<Button onClick={reportIssue}>{tc("settings:button:open issue")}</Button>
			</div>
			<div>
				<label className={"flex items-center gap-2"}>
					<Checkbox checked={releaseChannel == "beta"} onCheckedChange={(e) => changeReleaseChannel(e)}/>
					{tc("settings:receive beta updates")}
				</label>
				<p className={"text-sm whitespace-normal"}>{tc("settings:beta updates description")}</p>
			</div>
			{!isMac && <div>
				<label className={"flex items-center gap-2"}>
					<Checkbox checked={useAlcomForVccProtocol} onCheckedChange={(e) => changeUseAlcomForVcc(e)}/>
					{tc("settings:use alcom for vcc scheme")}
				</label>
				<Button className={"my-1"} disabled={!useAlcomForVccProtocol} onClick={installVccProtocol}>
					{tc("settings:register vcc scheme now")}
				</Button>
				<p className={"text-sm whitespace-normal"}>
					{tc(["settings:use vcc scheme description", "settings:vcc scheme description"])}
				</p>
			</div>}
			<p className={"whitespace-normal"}>
				{tc("settings:licenses description", {}, {
					components: {l: <Link href={"/settings/licenses"} className={"underline"}/>}
				})}
			</p>
		</Card>
	)
}
