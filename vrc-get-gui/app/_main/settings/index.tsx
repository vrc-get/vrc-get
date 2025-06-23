"use client";
import {
	queryOptions,
	useMutation,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { RefreshCw } from "lucide-react";
import { Suspense, useEffect, useTransition } from "react";
import Loading from "@/app/-loading";
import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import {
	BackupFormatSelect,
	BackupPathWarnings,
	FilePathRow,
	GuiAnimationSwitch,
	LanguageSelector,
	ProjectPathWarnings,
	ThemeSelector,
} from "@/components/common-setting-parts";
import { HNavBar, VStack } from "@/components/layout";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { ScrollPageContainer } from "@/components/ScrollPageContainer";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
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
import type { OpenOptions, UnityHubAccessMethod } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import globalInfo, { useGlobalInfo } from "@/lib/global-info";
import { tc, tt } from "@/lib/i18n";
import {
	toastError,
	toastNormal,
	toastSuccess,
	toastThrownError,
} from "@/lib/toast";
import { useEffectEvent } from "@/lib/use-effect-event";

export const Route = createFileRoute("/_main/settings/")({
	component: Page,
});

const environmentGetSettings = queryOptions({
	queryKey: ["environmentGetSettings"],
	queryFn: commands.environmentGetSettings,
});

function Page() {
	return (
		<VStack>
			<HNavBar
				className={"shrink-0"}
				leading={
					<p className="cursor-pointer py-1.5 font-bold grow-0">
						{tc("settings")}
					</p>
				}
			/>
			<Suspense
				fallback={
					<Card className={"p-4"}>
						<Loading loadingText={tc("general:loading...")} />
					</Card>
				}
			>
				<Settings />
			</Suspense>
		</VStack>
	);
}

function Settings() {
	const [updatingUnityPaths, updateUnityPathsTransition] = useTransition();

	const queryClient = useQueryClient();

	const updateUnityPaths = async () => {
		updateUnityPathsTransition(async () => {
			await commands.environmentUpdateUnityPathsFromUnityHub();
			await queryClient.invalidateQueries(environmentGetSettings);
		});
	};

	// at the time settings page is opened, unity hub path update might be in progress so we wait for it
	const waitForHubUpdate = useEffectEvent(async () => {
		updateUnityPathsTransition(async () => {
			await commands.environmentWaitForUnityHubUpdate();
			await queryClient.invalidateQueries(environmentGetSettings);
		});
	});
	useEffect(() => void waitForHubUpdate(), []);

	return (
		<ScrollPageContainer viewportClassName={"rounded-xl shadow-xl h-full"}>
			<main className="flex flex-col gap-2 shrink grow">
				<UnityHubPathCard updateUnityPaths={updateUnityPaths} />
				<UnityInstallationsCard
					updatingUnityPaths={updatingUnityPaths}
					updateUnityPaths={updateUnityPaths}
				/>
				<UnityLaunchArgumentsCard />
				<DefaultProjectPathCard />
				<BackupCard />
				<PackagesCard />
				<AppearanceCard />
				<FilesAndFoldersCard />
				<AlcomCard />
				<SystemInformationCard />
			</main>
		</ScrollPageContainer>
	);
}

function UnityHubPathCard({
	updateUnityPaths,
}: {
	updateUnityPaths: () => Promise<void>;
}) {
	const queryClient = useQueryClient();

	const {
		data: { unityHub },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			unityHub: data.unity_hub,
		}),
	});

	const pickUnityHub = useMutation({
		mutationFn: async () => await commands.environmentPickUnityHub(),
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
					toastSuccess(tc("settings:toast:unity hub path updated"));
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
			await updateUnityPaths();
		},
	});

	return (
		<Card className={"shrink-0 p-4"}>
			<h2 className={"pb-2"}>{tc("settings:unity hub path")}</h2>
			<FilePathRow
				path={unityHub}
				pick={pickUnityHub.mutate}
				notFoundMessage={"Unity Hub Not Found"}
				withOpen={false}
			/>
		</Card>
	);
}

function UnityInstallationsCard({
	updatingUnityPaths,
	updateUnityPaths,
}: {
	updatingUnityPaths: boolean;
	updateUnityPaths: () => void;
}) {
	const queryClient = useQueryClient();
	const {
		data: { unityPaths, unityHubAccessMethod },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			unityPaths: data.unity_paths,
			unityHubAccessMethod: data.unity_hub_access_method,
		}),
	});

	const addUnity = useMutation({
		mutationFn: async () => await commands.environmentPickUnity(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: (result) => {
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
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});
	const setAccessMethod = useMutation({
		mutationFn: async (method: UnityHubAccessMethod) =>
			await commands.environmentSetUnityHubAccessMethod(method),
		onMutate: async (method) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					unity_hub_access_method: method,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const UNITY_TABLE_HEAD = [
		"settings:unity:version",
		"settings:unity:path",
		"general:source",
	];

	return (
		<Card className={"shrink-0 p-4 flex flex-col gap-2"}>
			<div className={"flex align-middle"}>
				<div className={"grow flex items-center"}>
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
					onClick={() => addUnity.mutate()}
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
			<div>
				<label className={"flex items-center gap-2"}>
					<Checkbox
						checked={unityHubAccessMethod === "CallHub"}
						onCheckedChange={(e) =>
							setAccessMethod.mutate(e === true ? "CallHub" : "ReadConfig")
						}
					/>
					{tc("settings:use legacy unity hub loading")}
				</label>
				<p className={"text-sm whitespace-normal"}>
					{tc("settings:use legacy unity hub loading description")}
				</p>
			</div>
		</Card>
	);
}

function UnityLaunchArgumentsCard() {
	const { data: unityArgs } = useSuspenseQuery({
		...environmentGetSettings,
		select: (d) => d.default_unity_arguments,
	});

	const defaultUnityArgs = useGlobalInfo().defaultUnityArguments;
	const realUnityArgs = unityArgs ?? defaultUnityArgs;

	return (
		<Card className={"shrink-0 p-4"}>
			<div className={"mb-2 flex align-middle"}>
				<div className={"grow flex items-center"}>
					<h2>{tc("settings:default unity arguments")}</h2>
				</div>
				<Button
					onClick={async () => {
						try {
							await openSingleDialog(LaunchArgumentsEditDialogBody, {
								unityArgs,
							});
						} catch (e) {
							console.log(e);
							toastThrownError(e);
						}
					}}
					size={"sm"}
					className={"m-1"}
				>
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
		</Card>
	);
}

function LaunchArgumentsEditDialogBody({
	unityArgs,
	dialog,
}: {
	unityArgs: string[] | null;
	dialog: DialogContext<boolean>;
}) {
	const queryClient = useQueryClient();

	const setDefaultArgs = useMutation({
		mutationFn: async ({ value }: { value: string[] | null }) => {
			return await commands.environmentSetDefaultUnityArguments(value);
		},
		onMutate: async ({ value }) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					default_unity_arguments: value,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			dialog.error(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSuccess: () => {
			dialog.close(true);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const context = useUnityArgumentsSettings(
		unityArgs,
		globalInfo.defaultUnityArguments,
	);

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
				<Button onClick={() => dialog.close(false)} variant={"destructive"}>
					{tc("general:button:cancel")}
				</Button>
				<Button
					onClick={() =>
						void setDefaultArgs.mutate({ value: context.currentValue })
					}
					disabled={context.hasError}
				>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</>
	);
}

function DefaultProjectPathCard() {
	const queryClient = useQueryClient();

	const {
		data: { defaultProjectPath },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			defaultProjectPath: data.default_project_path,
		}),
	});

	const pickProjectDefaultPath = useMutation({
		mutationFn: async () => await commands.environmentPickProjectDefaultPath(),
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
					toastSuccess(tc("settings:toast:default project path updated"));
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	return (
		<Card className={"shrink-0 p-4"}>
			<h2 className={"mb-2"}>{tc("settings:default project path")}</h2>
			<p className={"whitespace-normal"}>
				{tc("settings:default project path description")}
			</p>
			<FilePathRow
				path={defaultProjectPath}
				pick={pickProjectDefaultPath.mutate}
			/>
			<ProjectPathWarnings projectPath={defaultProjectPath} />
		</Card>
	);
}

function BackupCard() {
	const queryClient = useQueryClient();

	const {
		data: { projectBackupPath, backupFormat, excludeVpmPackagesFromBackup },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			projectBackupPath: data.project_backup_path,
			backupFormat: data.backup_format,
			excludeVpmPackagesFromBackup: data.exclude_vpm_packages_from_backup,
		}),
	});

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
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const setExcludeVpmPackagesFromBackup = useMutation({
		mutationFn: async (flag: boolean) =>
			await commands.environmentSetExcludeVpmPackagesFromBackup(flag),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: (_, flag) => {
			queryClient.setQueryData(environmentGetSettings.queryKey, (old) => {
				if (old == null) return old;
				return { ...old, excludeVpmPackagesFromBackup: flag };
			});
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	return (
		<Card className={"shrink-0 p-4"}>
			<h2>{tc("projects:backup")}</h2>
			<div className="mt-2">
				<h3>{tc("settings:backup:path")}</h3>
				<p className={"whitespace-normal text-sm"}>
					{tc("settings:backup:path description")}
				</p>
				<FilePathRow
					path={projectBackupPath}
					pick={pickProjectBackupPath.mutate}
				/>
				<BackupPathWarnings backupPath={projectBackupPath} />
			</div>
			<div className="mt-2">
				<h3>{tc("settings:backup:format")}</h3>
				<p className={"whitespace-normal text-sm"}>
					{tc("settings:backup:format description")}
				</p>
				<label className={"flex items-center"}>
					<BackupFormatSelect backupFormat={backupFormat} />
				</label>
			</div>
			<div className="mt-2">
				<label className={"flex items-center gap-2"}>
					<Checkbox
						checked={excludeVpmPackagesFromBackup}
						onCheckedChange={(e) =>
							setExcludeVpmPackagesFromBackup.mutate(e === true)
						}
					/>
					{tc("settings:backup:exclude vpm packages from backup")}
				</label>
				<p className={"text-sm whitespace-normal"}>
					{tc("settings:backup:exclude vpm packages from backup description")}
				</p>
			</div>
		</Card>
	);
}

function PackagesCard() {
	const queryClient = useQueryClient();

	const {
		data: { showPrereleasePackages },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			showPrereleasePackages: data.show_prerelease_packages,
		}),
	});

	const clearPackageCache = useMutation({
		mutationFn: async () => await commands.environmentClearPackageCache(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: async () => {
			toastSuccess(tc("settings:toast:package cache cleared"));
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["environmentPackages"],
			});
		},
	});

	const setShowPrerelease = useMutation({
		mutationFn: async (showPrerelease: boolean) =>
			await commands.environmentSetShowPrereleasePackages(showPrerelease),
		onMutate: async (showPrerelease) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					show_prerelease_packages: showPrerelease,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	return (
		<Card className={"shrink-0 p-4 flex flex-col gap-4"}>
			<h2>{tc("settings:packages")}</h2>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button onClick={() => clearPackageCache.mutate()}>
					{tc("settings:clear package cache")}
				</Button>
			</div>
			<div>
				<label className={"flex items-center gap-2"}>
					<Checkbox
						checked={showPrereleasePackages}
						onCheckedChange={(e) => setShowPrerelease.mutate(e === true)}
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
		<Card className={"shrink-0 p-4"}>
			<h2>{tc("settings:appearance")}</h2>
			<LanguageSelector />
			<ThemeSelector />
			<GuiAnimationSwitch />
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
		<Card className={"shrink-0 p-4"}>
			<h2>{tc("settings:files and directories")}</h2>
			<p className={"mt-2"}>
				{tc("settings:files and directories:description")}
			</p>
			<div className={"flex flex-row flex-wrap gap-2"}>
				<Button
					className={"normal-case"}
					onClick={openVpmFolderContent("settings.json")}
				>
					{tc("settings:button:open settings.json")}
				</Button>
				<Button
					className={"normal-case"}
					onClick={openVpmFolderContent("vrc-get/gui-config.json")}
				>
					{tc("settings:button:open gui config.json")}
				</Button>
				<Button onClick={openVpmFolderContent("vrc-get/gui-logs")}>
					{tc("settings:button:open logs")}
				</Button>
				<Button
					onClick={openVpmFolderContent("Templates", "CreateFolderIfNotExists")}
				>
					{tc("settings:button:open vcc templates")}
				</Button>
			</div>
		</Card>
	);
}

function AlcomCard() {
	const globalInfo = useGlobalInfo();

	const queryClient = useQueryClient();

	const {
		data: { releaseChannel, useAlcomForVccProtocol },
	} = useSuspenseQuery({
		...environmentGetSettings,
		select: (data) => ({
			releaseChannel: data.release_channel,
			useAlcomForVccProtocol: data.use_alcom_for_vcc_protocol,
		}),
	});

	const setShowPrerelease = useMutation({
		mutationFn: async (releaseChannel: string) =>
			await commands.environmentSetReleaseChannel(releaseChannel),
		onMutate: async (releaseChannel) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					release_channel: releaseChannel,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const setUseAlcomForVccProtocol = useMutation({
		mutationFn: async (use: boolean) =>
			await commands.environmentSetUseAlcomForVccProtocol(use),
		onMutate: async (use) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					use_alcom_for_vcc_protocol: use,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const installVccProtocol = useMutation({
		mutationFn: async () => await commands.deepLinkInstallVcc(),
		onSuccess: () => {
			toastSuccess(tc("settings:toast:vcc scheme installed"));
		},
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
	});

	const checkForUpdate = async () => {
		try {
			const checkVersion = await commands.utilCheckForUpdate();
			if (checkVersion) {
				await openSingleDialog(CheckForUpdateMessage, {
					response: checkVersion,
				});
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

	return (
		<Card className={"shrink-0 p-4 flex flex-col gap-4"}>
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
							onCheckedChange={(value) =>
								setShowPrerelease.mutate(value === true ? "beta" : "stable")
							}
						/>
						{tc("settings:receive beta updates")}
					</label>
					<p className={"text-sm whitespace-normal"}>
						{tc("settings:beta updates description")}
					</p>
				</div>
			)}
			{globalInfo.shouldInstallDeepLink && (
				<div>
					<label className={"flex items-center gap-2"}>
						<Checkbox
							checked={useAlcomForVccProtocol}
							onCheckedChange={(value) =>
								setUseAlcomForVccProtocol.mutate(value === true)
							}
						/>
						{tc("settings:use alcom for vcc scheme")}
					</label>
					<Button
						className={"my-1"}
						disabled={!useAlcomForVccProtocol}
						onClick={() => installVccProtocol.mutate()}
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
							l: <Link to={"/settings/licenses"} className={"underline"} />,
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
		<Card className={"shrink-0 p-4 flex flex-col gap-4"}>
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
