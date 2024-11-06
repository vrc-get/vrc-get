"use client";

import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { ScrollPageContainer } from "@/components/ScrollPageContainer";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import {
	BackupFormatSelect,
	BackupPathWarnings,
	FilePathRow,
	LanguageSelector,
	ProjectPathWarnings,
	ThemeSelector,
} from "@/components/common-setting-parts";
import { HNavBar, VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	UnityArgumentsSettings,
	useUnityArgumentsSettings,
} from "@/components/unity-arguments-settings";
import { assertNever } from "@/lib/assert-never";
import type {
	CheckForUpdateResponse,
	OpenOptions,
	TauriEnvironmentSettings,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import globalInfo, { useGlobalInfo } from "@/lib/global-info";
import { tc, tt } from "@/lib/i18n";
import {
	toastError,
	toastNormal,
	toastSuccess,
	toastThrownError,
} from "@/lib/toast";
import { useFilePickerFunction } from "@/lib/use-file-picker-dialog";
import { useQuery } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import Link from "next/link";
import type React from "react";
import { useEffect } from "react";
import { useState } from "react";

export default function Page() {
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: commands.environmentGetSettings,
	});

	let body: React.ReactNode;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{tc("settings:error:load error")}</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>{tc("general:loading...")}</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch} />;
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

function Settings({
	settings,
	refetch,
}: {
	settings: TauriEnvironmentSettings;
	refetch: () => void;
}) {
	const isMac = useGlobalInfo().osType === "Darwin";

	const [updatingUnityPaths, setUpdatingUnityPaths] = useState(false);

	const updateUnityPaths = async () => {
		setUpdatingUnityPaths(true);
		try {
			await commands.environmentUpdateUnityPathsFromUnityHub();
			refetch();
		} finally {
			setUpdatingUnityPaths(false);
		}
	};

	// at the time settings page is opened, unity hub path update might be in progress so we wait for it
	// biome-ignore lint/correctness/useExhaustiveDependencies(refetch): we want to do on mount
	useEffect(() => {
		(async () => {
			setUpdatingUnityPaths(true);
			try {
				await commands.environmentWaitForUnityHubUpdate();
				refetch();
			} finally {
				setUpdatingUnityPaths(false);
			}
		})();
	}, []);

	return (
		<ScrollPageContainer>
			<main className="flex flex-col gap-2 flex-shrink flex-grow">
				<Card className={"flex-shrink-0 p-4"}>
					<h2 className={"pb-2"}>{tc("settings:unity hub path")}</h2>
					<FilePathRow
						withoutSelect
						path={settings.unity_hub}
						pick={commands.environmentPickUnityHub}
						refetch={() => {
							refetch();
							void updateUnityPaths();
						}}
						notFoundMessage={"Unity Hub Not Found"}
						successMessage={tc("settings:toast:unity hub path updated")}
					/>
				</Card>
				<UnityInstallationsCard
					refetch={refetch}
					updatingUnityPaths={updatingUnityPaths}
					updateUnityPaths={updateUnityPaths}
					unityPaths={settings.unity_paths}
				/>
				<UnityLaunchArgumentsCard
					refetch={refetch}
					unityArgs={settings.default_unity_arguments}
				/>
				<Card className={"flex-shrink-0 p-4"}>
					<h2 className={"mb-2"}>{tc("settings:default project path")}</h2>
					<p className={"whitespace-normal"}>
						{tc("settings:default project path description")}
					</p>
					<FilePathRow
						path={settings.default_project_path}
						pick={commands.environmentPickProjectDefaultPath}
						refetch={refetch}
						successMessage={tc("settings:toast:default project path updated")}
					/>
					<ProjectPathWarnings projectPath={settings.default_project_path} />
				</Card>
				<BackupCard
					projectBackupPath={settings.project_backup_path}
					backupFormat={settings.backup_format}
					refetch={refetch}
				/>
				<PackagesCard
					showPrereleasePackages={settings.show_prerelease_packages}
					refetch={refetch}
				/>
				<AppearanceCard />
				<FilesAndFoldersCard />
				<AlcomCard
					isMac={isMac}
					releaseChannel={settings.release_channel}
					useAlcomForVccProtocol={settings.use_alcom_for_vcc_protocol}
					refetch={refetch}
				/>
				<SystemInformationCard />
			</main>
		</ScrollPageContainer>
	);
}

function UnityInstallationsCard({
	refetch,
	unityPaths,
	updatingUnityPaths,
	updateUnityPaths,
}: {
	refetch: () => void;
	unityPaths: [path: string, version: string, fromHub: boolean][];
	updatingUnityPaths: boolean;
	updateUnityPaths: () => void;
}) {
	const [pickUnity, unityDialog] = useFilePickerFunction(
		commands.environmentPickUnity,
	);

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
					refetch();
					break;
				default:
					assertNever(result);
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const UNITY_TABLE_HEAD = [
		"settings:unity:version",
		"settings:unity:path",
		"general:source",
	];

	return (
		<Card className={"flex-shrink-0 p-4"}>
			<div className={"pb-2 flex align-middle"}>
				<div className={"flex-grow flex items-center"}>
					<h2>{tc("settings:unity installations")}</h2>
				</div>
				{updatingUnityPaths && (
					<div className={"flex items-center m-1"}>
						<Tooltip>
							<TooltipTrigger>
								<RefreshCw className="w-5 h-5 animate-spin" />
							</TooltipTrigger>
							<TooltipContent>
								{tc("settings:tooltip:reload unity from unity hub")}
							</TooltipContent>
						</Tooltip>
					</div>
				)}
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							disabled={updatingUnityPaths}
							onClick={updateUnityPaths}
							size={"sm"}
							className={"m-1"}
						>
							{tc("settings:button:reload unity from unity hub")}
						</Button>
					</TooltipTrigger>
					<TooltipContent>
						{tc("settings:tooltip:reload unity from unity hub")}
					</TooltipContent>
				</Tooltip>
				<Button
					disabled={updatingUnityPaths}
					onClick={addUnity}
					size={"sm"}
					className={"m-1"}
				>
					{tc("settings:button:add unity")}
				</Button>
			</div>
			<ScrollableCardTable
				className={`w-full min-h-[20vh] ${updatingUnityPaths ? "opacity-50" : ""}`}
			>
				<thead>
					<tr>
						{UNITY_TABLE_HEAD.map((head, index) => (
							<th
								// biome-ignore lint/suspicious/noArrayIndexKey: static array
								key={index}
								className={
									"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
								}
							>
								<small className="font-normal leading-none">{tc(head)}</small>
							</th>
						))}
					</tr>
				</thead>
				<tbody>
					{unityPaths.map(([path, version, isFromHub]) => (
						<tr key={path} className="even:bg-secondary/30">
							<td className={"p-2.5"}>{version}</td>
							<td className={"p-2.5"}>{path}</td>
							<td className={"p-2.5"}>
								{isFromHub
									? tc("settings:unity:source:unity hub")
									: tc("settings:unity:source:manual")}
							</td>
						</tr>
					))}
				</tbody>
			</ScrollableCardTable>
			{unityDialog}
		</Card>
	);
}

function UnityLaunchArgumentsCard({
	refetch,
	unityArgs,
}: {
	refetch: () => void;
	unityArgs: string[] | null;
}) {
	const [open, setOpen] = useState(false);

	const defaultUnityArgs = useGlobalInfo().defaultUnityArguments;
	const realUnityArgs = unityArgs ?? defaultUnityArgs;

	const close = () => setOpen(false);
	const openDialog = () => setOpen(true);

	return (
		<Card className={"flex-shrink-0 p-4"}>
			<div className={"mb-2 flex align-middle"}>
				<div className={"flex-grow flex items-center"}>
					<h2>{tc("settings:default unity arguments")}</h2>
				</div>
				<Button onClick={openDialog} size={"sm"} className={"m-1"}>
					{tc("general:button:edit")}
				</Button>
			</div>
			<p className={"whitespace-normal"}>
				{tc("settings:default unity arguments description")}
			</p>
			<ol className={"flex flex-col"}>
				{realUnityArgs.map((v, i) => (
					<Input disabled key={i + v} value={v} className={"w-full"} />
				))}
			</ol>
			{open && (
				<DialogOpen>
					<LaunchArgumentsEditDialogBody
						unityArgs={unityArgs}
						refetch={refetch}
						close={close}
					/>
				</DialogOpen>
			)}
		</Card>
	);
}

function LaunchArgumentsEditDialogBody({
	unityArgs,
	refetch,
	close,
}: {
	unityArgs: string[] | null;
	refetch: () => void;
	close: () => void;
}) {
	const context = useUnityArgumentsSettings(
		unityArgs,
		globalInfo.defaultUnityArguments,
	);

	const saveAndClose = async () => {
		await commands.environmentSetDefaultUnityArguments(context.currentValue);
		close();
		refetch();
	};

	return (
		<>
			<DialogTitle>
				{tc("settings:dialog:default launch arguments")}
			</DialogTitle>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50dvh] overflow-y-auto"}>
				<UnityArgumentsSettings context={context} />
			</DialogDescription>
			<DialogFooter>
				<Button onClick={close} variant={"destructive"}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={saveAndClose} disabled={context.hasError}>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</>
	);
}

function BackupCard({
	projectBackupPath,
	backupFormat,
	refetch,
}: {
	projectBackupPath: string;
	backupFormat: string;
	refetch: () => void;
}) {
	const setBackupFormat = async (format: string) => {
		try {
			await commands.environmentSetBackupFormat(format);
			refetch();
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	return (
		<Card className={"flex-shrink-0 p-4"}>
			<h2>{tc("projects:backup")}</h2>
			<div className="mt-2">
				<h3>{tc("settings:backup:path")}</h3>
				<p className={"whitespace-normal text-sm"}>
					{tc("settings:backup:path description")}
				</p>
				<FilePathRow
					path={projectBackupPath}
					pick={commands.environmentPickProjectBackupPath}
					refetch={refetch}
					successMessage={tc("settings:toast:backup path updated")}
				/>
				<BackupPathWarnings backupPath={projectBackupPath} />
			</div>
			<div className="mt-2">
				<h3>{tc("settings:backup:format")}</h3>
				<p className={"whitespace-normal text-sm"}>
					{tc("settings:backup:format description")}
				</p>
				<label className={"flex items-center"}>
					<BackupFormatSelect
						backupFormat={backupFormat}
						setBackupFormat={setBackupFormat}
					/>
				</label>
			</div>
		</Card>
	);
}

function PackagesCard({
	showPrereleasePackages,
	refetch,
}: {
	showPrereleasePackages: boolean;
	refetch: () => void;
}) {
	const clearPackageCache = async () => {
		try {
			await commands.environmentClearPackageCache();
			toastSuccess(tc("settings:toast:package cache cleared"));
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const toggleShowPrereleasePackages = async (e: "indeterminate" | boolean) => {
		try {
			await commands.environmentSetShowPrereleasePackages(e === true);
			refetch();
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	return (
		<Card className={"flex-shrink-0 p-4 flex flex-col gap-4"}>
			<h2>{tc("settings:packages")}</h2>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button onClick={clearPackageCache}>
					{tc("settings:clear package cache")}
				</Button>
			</div>
			<div>
				<label className={"flex items-center gap-2"}>
					<Checkbox
						checked={showPrereleasePackages}
						onCheckedChange={(e) => toggleShowPrereleasePackages(e)}
					/>
					{tc("settings:show prerelease")}
				</label>
				<p className={"text-sm whitespace-normal"}>
					{tc("settings:show prerelease description")}
				</p>
			</div>
		</Card>
	);
}

function AppearanceCard() {
	return (
		<Card className={"flex-shrink-0 p-4"}>
			<h2>{tc("settings:appearance")}</h2>
			<LanguageSelector />
			<ThemeSelector />
		</Card>
	);
}

function FilesAndFoldersCard() {
	const openVpmFolderContent = (
		subPath: string,
		ifNotExists: OpenOptions = "ErrorIfNotExists",
	) => {
		return async () => {
			try {
				await commands.utilOpen(
					`${globalInfo.vpmHomeFolder}/${subPath}`,
					ifNotExists,
				);
			} catch (e) {
				console.error(e);
				toastThrownError(e);
			}
		};
	};

	return (
		<Card className={"flex-shrink-0 p-4"}>
			<h2>{tc("settings:files and directories")}</h2>
			<p className={"mt-2"}>
				{tc("settings:files and directories:description")}
			</p>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button onClick={openVpmFolderContent("settings.json")}>
					{tc("settings:button:open settings.json")}
				</Button>
				<Button onClick={openVpmFolderContent("vrc-get/gui-config.json")}>
					{tc("settings:button:open gui config.json")}
				</Button>
				<Button onClick={openVpmFolderContent("vrc-get/gui-logs")}>
					{tc("settings:button:open logs")}
				</Button>
				<Button
					onClick={openVpmFolderContent("Templates", "CreateFolderIfNotExists")}
				>
					{tc("settings:button:open custom templates")}
				</Button>
			</div>
		</Card>
	);
}

function AlcomCard({
	isMac,
	releaseChannel,
	useAlcomForVccProtocol,
	refetch,
}: {
	isMac: boolean;
	releaseChannel: string;
	useAlcomForVccProtocol: boolean;
	refetch: () => void;
}) {
	const [updateState, setUpdateState] = useState<CheckForUpdateResponse | null>(
		null,
	);
	const globalInfo = useGlobalInfo();

	const checkForUpdate = async () => {
		try {
			const checkVersion = await commands.utilCheckForUpdate();
			if (checkVersion) {
				setUpdateState(checkVersion);
			} else {
				toastNormal(tc("check update:toast:no updates"));
			}
		} catch (e) {
			toastThrownError(e);
			console.error(e);
		}
	};

	const reportIssue = async () => {
		const url = new URL("https://github.com/vrc-get/vrc-get/issues/new");
		url.searchParams.append("labels", "bug,vrc-get-gui");
		url.searchParams.append("template", "01_gui_bug-report.yml");
		url.searchParams.append("os", `${globalInfo.osInfo} - ${globalInfo.arch}`);
		url.searchParams.append("webview-version", `${globalInfo.webviewVersion}`);
		let version = globalInfo.version ?? "unknown";
		if (globalInfo.commitHash) {
			version += ` (${globalInfo.commitHash})`;
		} else {
			version += " (unknown commit)";
		}
		url.searchParams.append("version", version);

		void commands.utilOpenUrl(url.toString());
	};

	const changeReleaseChannel = async (value: "indeterminate" | boolean) => {
		await commands.environmentSetReleaseChannel(
			value === true ? "beta" : "stable",
		);
		refetch();
	};

	const changeUseAlcomForVcc = async (value: "indeterminate" | boolean) => {
		await commands.environmentSetUseAlcomForVccProtocol(value === true);
		refetch();
	};

	const installVccProtocol = async () => {
		try {
			await commands.deepLinkInstallVcc();
			toastSuccess(tc("settings:toast:vcc scheme installed"));
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	return (
		<Card className={"flex-shrink-0 p-4 flex flex-col gap-4"}>
			{updateState && (
				<CheckForUpdateMessage
					response={updateState}
					close={() => setUpdateState(null)}
				/>
			)}
			<h2>ALCOM</h2>
			<div className={"flex flex-row flex-wrap gap-2"}>
				{globalInfo.checkForUpdates && (
					<Button onClick={checkForUpdate}>
						{tc("settings:check update")}
					</Button>
				)}
				<Button onClick={reportIssue}>
					{tc("settings:button:open issue")}
				</Button>
			</div>
			{globalInfo.checkForUpdates && (
				<div>
					<label className={"flex items-center gap-2"}>
						<Checkbox
							checked={releaseChannel === "beta"}
							onCheckedChange={(e) => changeReleaseChannel(e)}
						/>
						{tc("settings:receive beta updates")}
					</label>
					<p className={"text-sm whitespace-normal"}>
						{tc("settings:beta updates description")}
					</p>
				</div>
			)}
			{!isMac && (
				<div>
					<label className={"flex items-center gap-2"}>
						<Checkbox
							checked={useAlcomForVccProtocol}
							onCheckedChange={(e) => changeUseAlcomForVcc(e)}
						/>
						{tc("settings:use alcom for vcc scheme")}
					</label>
					<Button
						className={"my-1"}
						disabled={!useAlcomForVccProtocol}
						onClick={installVccProtocol}
					>
						{tc("settings:register vcc scheme now")}
					</Button>
					<p className={"text-sm whitespace-normal"}>
						{tc([
							"settings:use vcc scheme description",
							"settings:vcc scheme description",
						])}
					</p>
				</div>
			)}
			<p className={"whitespace-normal"}>
				{tc(
					"settings:licenses description",
					{},
					{
						components: {
							l: <Link href={"/settings/licenses"} className={"underline"} />,
						},
					},
				)}
			</p>
		</Card>
	);
}

function SystemInformationCard() {
	const info = useGlobalInfo();

	return (
		<Card className={"flex-shrink-0 p-4 flex flex-col gap-4"}>
			<h2>{tc("settings:system information")}</h2>
			<dl>
				<dt>{tc("settings:os")}</dt>
				<dd className={"opacity-50 mb-2"}>{info.osInfo}</dd>
				<dt>{tc("settings:architecture")}</dt>
				<dd className={"opacity-50 mb-2"}>{info.arch}</dd>
				<dt>{tc("settings:webview version")}</dt>
				<dd className={"opacity-50 mb-2"}>{info.webviewVersion}</dd>
				<dt>{tc("settings:alcom version")}</dt>
				<dd className={"opacity-50 mb-2"}>{info.version}</dd>
				<dt>{tc("settings:alcom commit hash")}</dt>
				<dd className={"opacity-50 mb-2"}>{info.commitHash}</dd>
			</dl>
		</Card>
	);
}
