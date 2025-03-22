"use client";

import Loading from "@/app/-loading";
import { CheckForUpdateMessage } from "@/components/CheckForUpdateMessage";
import { ScrollPageContainer } from "@/components/ScrollPageContainer";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
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
import type {
	OpenOptions,
	TauriEnvironmentSettings,
	UnityHubAccessMethod,
} from "@/lib/bindings";
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
import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { RefreshCw } from "lucide-react";
import type React from "react";
import { useTransition } from "react";
import { useEffect } from "react";

export const Route = createFileRoute("/_main/settings/")({
	component: Page,
});

const environmentGetSettings = queryOptions({
	queryKey: ["environmentGetSettings"],
	queryFn: commands.environmentGetSettings,
});

function Page() {
	const result = useQuery(environmentGetSettings);

	let body: React.ReactNode;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{tc("settings:error:load error")}</Card>;
			break;
		case "pending":
			body = (
				<Card className={"p-4"}>
					<Loading loadingText={tc("general:loading...")} />
				</Card>
			);
			break;
		case "success":
			body = <Settings settings={result.data} />;
			break;
		default:
			assertNever(result);
	}

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
			{body}
		</VStack>
	);
}

function Settings({
	settings,
}: {
	settings: TauriEnvironmentSettings;
}) {
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
		<ScrollPageContainer viewportClassName={"rounded-xl shadow-xl h-full"}>
			<main className="flex flex-col gap-2 shrink grow">
				<Card className={"shrink-0 p-4"}>
					<h2 className={"pb-2"}>{tc("settings:unity hub path")}</h2>
					<FilePathRow
						path={settings.unity_hub}
						pick={pickUnityHub.mutate}
						notFoundMessage={"Unity Hub Not Found"}
						withOpen={false}
					/>
				</Card>
				<UnityInstallationsCard
					updatingUnityPaths={updatingUnityPaths}
					updateUnityPaths={updateUnityPaths}
					unityPaths={settings.unity_paths}
					unityHubAccessMethod={settings.unity_hub_access_method}
				/>
				<UnityLaunchArgumentsCard
					unityArgs={settings.default_unity_arguments}
				/>
				<Card className={"shrink-0 p-4"}>
					<h2 className={"mb-2"}>{tc("settings:default project path")}</h2>
					<p className={"whitespace-normal"}>
						{tc("settings:default project path description")}
					</p>
					<FilePathRow
						path={settings.default_project_path}
						pick={pickProjectDefaultPath.mutate}
					/>
					<ProjectPathWarnings projectPath={settings.default_project_path} />
				</Card>
				<BackupCard
					projectBackupPath={settings.project_backup_path}
					backupFormat={settings.backup_format}
				/>
				<PackagesCard
					showPrereleasePackages={settings.show_prerelease_packages}
				/>
				<AppearanceCard />
				<FilesAndFoldersCard />
				<AlcomCard
					releaseChannel={settings.release_channel}
					useAlcomForVccProtocol={settings.use_alcom_for_vcc_protocol}
				/>
				<SystemInformationCard />
			</main>
		</ScrollPageContainer>
	);
}

function UnityInstallationsCard({
	unityPaths,
	unityHubAccessMethod,
	updatingUnityPaths,
	updateUnityPaths,
}: {
	unityPaths: [path: string, version: string, fromHub: boolean][];
	unityHubAccessMethod: UnityHubAccessMethod;
	updatingUnityPaths: boolean;
	updateUnityPaths: () => void;
}) {
	const queryClient = useQueryClient();
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

function UnityLaunchArgumentsCard({
	unityArgs,
}: {
	unityArgs: string[] | null;
}) {
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

function BackupCard({
	projectBackupPath,
	backupFormat,
}: {
	projectBackupPath: string;
	backupFormat: string;
}) {
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
		</Card>
	);
}

function PackagesCard({
	showPrereleasePackages,
}: {
	showPrereleasePackages: boolean;
}) {
	const queryClient = useQueryClient();
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
					{tc("settings:button:open custom templates")}
				</Button>
			</div>
		</Card>
	);
}

function AlcomCard({
	releaseChannel,
	useAlcomForVccProtocol,
}: {
	releaseChannel: string;
	useAlcomForVccProtocol: boolean;
}) {
	const globalInfo = useGlobalInfo();

	const queryClient = useQueryClient();
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
