"use client";

import {
	queryOptions,
	type UseQueryResult,
	useIsMutating,
	useMutation,
	useQueries,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import {
	createFileRoute,
	useNavigate,
	useRouter,
} from "@tanstack/react-router";
import { ArrowLeft, ChevronDown } from "lucide-react";
import type React from "react";
import { Suspense, useMemo } from "react";
import { copyProject } from "@/app/_main/projects/manage/-copy-project";
import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { HNavBar, VStack } from "@/components/layout";
import { OpenUnityButton } from "@/components/OpenUnityButton";
import { RemoveProjectDialog } from "@/components/RemoveProjectDialog";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	UnityArgumentsSettings,
	useUnityArgumentsSettings,
} from "@/components/unity-arguments-settings";
import type { TauriProjectDetails, TauriUnityVersions } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { VRCSDK_PACKAGES, VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { nameFromPath } from "@/lib/os";
import { toastSuccess, toastThrownError } from "@/lib/toast";
import { compareUnityVersionString, parseUnityVersion } from "@/lib/version";
import { combinePackagesAndProjectDetails } from "./-collect-package-row-info";
import { PackageListCard } from "./-package-list-card";
import { PageContextProvider } from "./-page-context";
import { unityVersionChange } from "./-unity-migration";
import { applyChangesMutation } from "./-use-package-change";

interface SearchParams {
	projectPath: string;
}

export const Route = createFileRoute("/_main/projects/manage/")({
	component: Page,
	validateSearch: (a): SearchParams => ({
		projectPath: a.projectPath == null ? "" : `${a.projectPath}`,
	}),
});

function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

function PageBody() {
	const { projectPath } = Route.useSearch();
	const router = useRouter();

	// repositoriesInfo: list of repositories and their visibility
	// packagesResult: list of packages
	// detailsResult: project details including installed packages
	// unityVersionsResult: list of unity versions installed
	const [repositoriesInfo, packagesResult, detailsResult, unityVersionsResult] =
		useQueries({
			queries: [
				{
					queryKey: ["environmentRepositoriesInfo"],
					queryFn: commands.environmentRepositoriesInfo,
					refetchOnWindowFocus: false,
				},
				{
					queryKey: ["environmentPackages"],
					queryFn: commands.environmentPackages,
					refetchOnWindowFocus: false,
				},
				{
					queryKey: ["projectDetails", projectPath],
					queryFn: () => commands.projectDetails(projectPath),
					refetchOnWindowFocus: false,
				},
				{
					queryKey: ["environmentUnityVersions"],
					queryFn: () => commands.environmentUnityVersions(),
				},
			],
		});

	const packageRowsData = useMemo(() => {
		const packages = packagesResult.data ?? [];
		const details = detailsResult.data ?? null;
		const hiddenRepositories =
			repositoriesInfo.data?.hidden_user_repositories ?? [];
		const hideUserPackages =
			repositoriesInfo.data?.hide_local_user_packages ?? false;
		const definedRepositories = repositoriesInfo.data?.user_repositories ?? [];
		const showPrereleasePackages =
			repositoriesInfo.data?.show_prerelease_packages ?? false;
		return combinePackagesAndProjectDetails(
			packages,
			details,
			hiddenRepositories,
			hideUserPackages,
			definedRepositories,
			showPrereleasePackages,
		);
	}, [repositoriesInfo.data, packagesResult.data, detailsResult.data]);

	const queryClient = useQueryClient();

	const refetchPackages = useMutation({
		mutationFn: async () => await commands.environmentRefetchPackages(),
		onError: (e) => {
			reportError(e);
			console.error(e);
		},
		onSettled: async () => {
			await Promise.all([
				queryClient.invalidateQueries({
					queryKey: ["environmentRepositoriesInfo"],
				}),
				queryClient.invalidateQueries({ queryKey: ["environmentPackages"] }),
				queryClient.invalidateQueries({
					queryKey: ["projectDetails", projectPath],
				}),
				queryClient.invalidateQueries({
					queryKey: ["environmentUnityVersions"],
				}),
			]);
		},
	});

	const fetchingMutation = useIsMutating({
		mutationKey: applyChangesMutation(projectPath).mutationKey,
	});

	const requestChangeUnityVersion = (
		version: string,
		mayUseChinaVariant?: boolean,
	) => {
		if (detailsResult.data == null)
			throw new Error("Project details not ready");
		const isVRCProject = detailsResult.data.installed_packages.some(([id, _]) =>
			VRCSDK_PACKAGES.includes(id),
		);
		void unityVersionChange({
			projectPath,
			version,
			isVRCProject,
			currentUnityVersion: detailsResult.data.unity_str ?? "unknown",
			mayUseChinaVariant,
			navigate: router.navigate,
		});
	};

	const isLoading =
		packagesResult.isFetching ||
		detailsResult.isFetching ||
		repositoriesInfo.isFetching ||
		unityVersionsResult.isLoading ||
		fetchingMutation !== 0 ||
		refetchPackages.isPending;

	console.log(`rerender: isloading: ${isLoading}`);

	const pageContext = useMemo(() => ({ isLoading }), [isLoading]);

	return (
		<PageContextProvider value={pageContext}>
			<VStack>
				<ProjectViewHeader
					className={"shrink-0 compact:py-0"}
					isLoading={isLoading}
					detailsResult={detailsResult}
					unityVersionsResult={unityVersionsResult}
					requestChangeUnityVersion={requestChangeUnityVersion}
				/>
				{detailsResult?.data?.should_resolve && (
					<SuggestResolveProjectCard disabled={isLoading} />
				)}
				<MigrationCards
					isLoading={isLoading}
					detailsResult={detailsResult.data}
					unityVersionsResult={unityVersionsResult.data}
					requestChangeUnityVersion={requestChangeUnityVersion}
				/>
				<main className="shrink overflow-hidden flex w-full h-full">
					<PackageListCard
						packageRowsData={packageRowsData}
						repositoriesInfo={repositoriesInfo.data}
						onRefresh={() => refetchPackages.mutate()}
					/>
				</main>
			</VStack>
		</PageContextProvider>
	);
}

function UnityVersionSelector({
	disabled,
	detailsResult,
	requestChangeUnityVersion,
	unityVersions,
}: {
	disabled?: boolean;
	detailsResult: UseQueryResult<TauriProjectDetails>;
	requestChangeUnityVersion: (version: string) => void;
	unityVersions?: TauriUnityVersions;
}) {
	const unityVersionNames = useMemo(() => {
		if (unityVersions == null) return null;
		const versionNames = [
			...new Set<string>(unityVersions.unity_paths.map(([, path]) => path)),
		];
		versionNames.sort((a, b) => compareUnityVersionString(b, a));
		return versionNames;
	}, [unityVersions]);

	const isVRCProject =
		detailsResult.data?.installed_packages.some(([id, _]) =>
			VRCSDK_PACKAGES.includes(id),
		) ?? false;

	let unityVersionList: React.ReactNode;

	if (unityVersionNames == null) {
		unityVersionList = <SelectLabel>Loading...</SelectLabel>;
	} else if (isVRCProject) {
		const vrcSupportedVersions = unityVersionNames.filter((v) =>
			VRCSDK_UNITY_VERSIONS.includes(v),
		);
		const vrcUnsupportedVersions = unityVersionNames.filter(
			(v) => !VRCSDK_UNITY_VERSIONS.includes(v),
		);

		if (
			vrcUnsupportedVersions.length === 0 ||
			vrcUnsupportedVersions.length === 0
		) {
			unityVersionList = unityVersionNames.map((v) => (
				<SelectItem key={v} value={v}>
					{v}
				</SelectItem>
			));
		} else {
			// if there are both supported and unsupported versions, show them separately
			unityVersionList = (
				<>
					{vrcSupportedVersions.map((v) => (
						<SelectItem key={v} value={v}>
							{v}
						</SelectItem>
					))}
					<SelectLabel>
						<Separator className={"-ml-6 mr-0 w-auto"} />
					</SelectLabel>
					{vrcUnsupportedVersions.map((v) => (
						<SelectItem key={v} value={v}>
							{v}
						</SelectItem>
					))}
				</>
			);
		}
	} else {
		unityVersionList = unityVersionNames.map((v) => (
			<SelectItem key={v} value={v}>
				{v}
			</SelectItem>
		));
	}

	return (
		<Select
			disabled={disabled}
			value={detailsResult.data?.unity_str ?? undefined}
			onValueChange={requestChangeUnityVersion}
		>
			<SelectTrigger>
				{detailsResult.status === "success" ? (
					(detailsResult.data.unity_str ?? "unknown")
				) : (
					<span className={"text-primary"}>Loading...</span>
				)}
			</SelectTrigger>
			<SelectContent>
				<SelectGroup>{unityVersionList}</SelectGroup>
			</SelectContent>
		</Select>
	);
}

function SuggestResolveProjectCard({ disabled }: { disabled?: boolean }) {
	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

	return (
		<Card className={"shrink-0 p-2 flex flex-row items-center compact:p-1"}>
			<p className="cursor-pointer py-1.5 font-bold grow-0 shrink overflow-hidden whitespace-normal text-sm pl-2">
				{tc("projects:manage:suggest resolve")}
			</p>
			<div className={"grow shrink-0 w-2"} />
			<Button
				variant={"ghost-destructive"}
				onClick={() => packageChange.mutate({ type: "resolve" })}
				disabled={disabled}
			>
				{tc("projects:manage:button:resolve")}
			</Button>
		</Card>
	);
}

function MigrationCards({
	isLoading,
	detailsResult,
	unityVersionsResult,
	requestChangeUnityVersion,
}: {
	isLoading: boolean;
	detailsResult?: TauriProjectDetails;
	unityVersionsResult?: TauriUnityVersions;
	requestChangeUnityVersion: (
		version: string,
		keepChinaVariant?: boolean,
	) => void;
}) {
	if (detailsResult == null) return null;
	if (unityVersionsResult == null) return null;
	if (detailsResult.unity == null) return false;
	if (detailsResult.unity_str == null) return false;
	const currentUnity = detailsResult.unity_str;

	const isVRChatProject = detailsResult.installed_packages.some(([id, _]) =>
		VRCSDK_PACKAGES.includes(id),
	);

	// we only migrate VRChat project (for now)
	if (!isVRChatProject) return null;

	// for 2019 projects, VRChat recommends migrating to 2022
	const isMigrationTo2022Recommended = detailsResult.unity[0] === 2019;
	const is2022PatchMigrationRecommended =
		detailsResult.unity[0] === 2022 &&
		compareUnityVersionString(
			detailsResult.unity_str,
			unityVersionsResult.recommended_version,
		) !== 0;

	const isChinaToInternationalMigrationRecommended =
		parseUnityVersion(detailsResult.unity_str)?.chinaIncrement != null;

	return (
		<>
			{isMigrationTo2022Recommended && (
				<SuggestMigrateTo2022Card
					disabled={isLoading}
					onMigrateRequested={() =>
						requestChangeUnityVersion(
							unityVersionsResult.recommended_version,
							true,
						)
					}
				/>
			)}
			{is2022PatchMigrationRecommended && (
				<Suggest2022PatchMigrationCard
					disabled={isLoading}
					onMigrateRequested={() =>
						requestChangeUnityVersion(
							unityVersionsResult.recommended_version,
							true,
						)
					}
				/>
			)}
			{isChinaToInternationalMigrationRecommended && (
				<SuggestChinaToInternationalMigrationCard
					disabled={isLoading}
					onMigrateRequested={() => {
						const internationalVersion = currentUnity.slice(
							0,
							currentUnity.indexOf("c"),
						);
						requestChangeUnityVersion(internationalVersion);
					}}
				/>
			)}
		</>
	);
}

function SuggestMigrateTo2022Card({
	disabled,
	onMigrateRequested,
}: {
	disabled?: boolean;
	onMigrateRequested: () => void;
}) {
	return (
		<Card className={"shrink-0 p-2 flex flex-row items-center compact:p-1"}>
			<p className="cursor-pointer py-1.5 font-bold grow-0 shrink overflow-hidden whitespace-normal text-sm pl-2">
				{tc("projects:manage:suggest unity migration")}
			</p>
			<div className={"grow shrink-0 w-2"} />
			<Button
				variant={"ghost-destructive"}
				onClick={onMigrateRequested}
				disabled={disabled}
			>
				{tc("projects:manage:button:unity migrate")}
			</Button>
		</Card>
	);
}

function Suggest2022PatchMigrationCard({
	disabled,
	onMigrateRequested,
}: {
	disabled?: boolean;
	onMigrateRequested: () => void;
}) {
	return (
		<Card className={"shrink-0 p-2 flex flex-row items-center compact:p-1"}>
			<p className="cursor-pointer py-1.5 font-bold grow-0 shrink overflow-hidden whitespace-normal text-sm pl-2">
				{tc("projects:manage:suggest unity patch migration")}
			</p>
			<div className={"grow shrink-0 w-2"} />
			<Button
				variant={"ghost-destructive"}
				onClick={onMigrateRequested}
				disabled={disabled}
			>
				{tc("projects:manage:button:unity migrate")}
			</Button>
		</Card>
	);
}

function SuggestChinaToInternationalMigrationCard({
	disabled,
	onMigrateRequested,
}: {
	disabled?: boolean;
	onMigrateRequested: () => void;
}) {
	return (
		<Card className={"shrink-0 p-2 flex flex-row items-center compact:p-1"}>
			<p className="cursor-pointer py-1.5 font-bold grow-0 shrink overflow-hidden whitespace-normal text-sm pl-2">
				{tc("projects:manage:suggest unity china to international migration")}
			</p>
			<div className={"grow shrink-0 w-2"} />
			<Button
				variant={"ghost-destructive"}
				onClick={onMigrateRequested}
				disabled={disabled}
			>
				{tc("projects:manage:button:unity migrate")}
			</Button>
		</Card>
	);
}

function ProjectViewHeader({
	className,
	isLoading,
	detailsResult,
	unityVersionsResult,
	requestChangeUnityVersion,
}: {
	className?: string;
	isLoading: boolean | undefined;
	detailsResult: UseQueryResult<TauriProjectDetails, Error>;
	unityVersionsResult: UseQueryResult<TauriUnityVersions, Error>;
	requestChangeUnityVersion: (
		version: string,
		mayUseChinaVariant?: boolean,
	) => void;
}) {
	const { projectPath } = Route.useSearch();
	const projectName = nameFromPath(projectPath);

	return (
		<HNavBar
			className={`${className}`}
			commonClassName={"min-h-12"}
			leadingClassName="compact:-ml-2.5"
			trailingClassName="compact:-mr-2"
			leading={
				<>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant={"ghost"}
								size={"sm"}
								onClick={() => history.back()}
							>
								<ArrowLeft className={"w-5 h-5"} />
							</Button>
						</TooltipTrigger>
						<TooltipContent>
							{tc("projects:manage:tooltip:back to projects")}
						</TooltipContent>
					</Tooltip>

					<div className={"pl-2 space-y-0 my-1 shrink min-w-0 compact:pl-0"}>
						<p className="cursor-pointer font-bold grow-0 whitespace-pre mb-0 leading-tight">
							{projectName}
						</p>
						<p className="cursor-pointer text-sm leading-tight mt-0">
							{tc(
								"projects:manage:project location",
								{ path: projectPath },
								{
									components: {
										path: (
											<span
												className={
													"p-0.5 font-path whitespace-pre bg-secondary text-secondary-foreground"
												}
											/>
										),
									},
								},
							)}
						</p>
					</div>
				</>
			}
			trailing={
				<>
					<div className="flex items-center gap-1">
						<p className="cursor-pointer py-1.5 font-bold">
							{tc("projects:manage:unity version")}
						</p>
						<div className={"flex"}>
							<UnityVersionSelector
								disabled={isLoading}
								detailsResult={detailsResult}
								unityVersions={unityVersionsResult.data}
								requestChangeUnityVersion={requestChangeUnityVersion}
							/>
						</div>
					</div>
					<div className={"grow-0 shrink-0 w-max"}>
						<ProjectButton
							projectPath={projectPath}
							unityVersion={detailsResult.data?.unity_str ?? null}
							unityRevision={detailsResult.data?.unity_revision ?? null}
						/>
					</div>
				</>
			}
		/>
	);
}

function LaunchSettings({
	defaultUnityArgs,
	initialValue,
	dialog,
}: {
	defaultUnityArgs: string[];
	initialValue: string[] | null;
	dialog: DialogContext<string[] | null | false>;
}) {
	const context = useUnityArgumentsSettings(initialValue, defaultUnityArgs);

	const saveAndClose = async () => {
		dialog.close(context.currentValue);
	};

	return (
		<>
			<DialogTitle>{tc("projects:dialog:launch options")}</DialogTitle>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50dvh] overflow-y-auto"}>
				<h3 className={"text-lg"}>
					{tc("projects:dialog:command-line arguments")}
				</h3>
				<UnityArgumentsSettings context={context} />
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close(false)} variant={"destructive"}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={saveAndClose} disabled={context.hasError}>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</>
	);
}

function projectGetUnityPath(projectPath: string) {
	return queryOptions({
		queryFn: () => commands.projectGetUnityPath(projectPath),
		queryKey: ["projectGetUnityPath", projectPath],
		refetchOnWindowFocus: false,
	});
}

function DropdownMenuContentBody({
	projectPath,
	removeProject,
	onChangeLaunchOptions,
}: {
	projectPath: string;
	removeProject?: () => void;
	onChangeLaunchOptions?: () => void;
}) {
	const openProjectFolder = () =>
		commands.utilOpen(projectPath, "ErrorIfNotExists");

	const queryClient = useQueryClient();
	const setUnityPath = useMutation({
		mutationFn: async (unityPath: string | null) =>
			await commands.projectSetUnityPath(projectPath, unityPath),
		onMutate: async (unityPath) => {
			const getUnityPath = projectGetUnityPath(projectPath);
			await queryClient.invalidateQueries(getUnityPath);
			const data = queryClient.getQueryData(getUnityPath.queryKey);
			queryClient.setQueryData(getUnityPath.queryKey, unityPath);
			return data;
		},
		onError: (e, _, data) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(projectGetUnityPath(projectPath).queryKey, data);
		},
		onSuccess: () => {
			toastSuccess(tc("projects:toast:forgot unity path"));
		},
	});
	const unityPathQuery = useQuery(projectGetUnityPath(projectPath));

	const navigate = useNavigate();
	const onCopyProject = async () => {
		try {
			await copyProject(projectPath, navigate);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const onBackup = async () => {
		try {
			await openSingleDialog(BackupProjectDialog, {
				projectPath,
			});
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const unityPath = unityPathQuery.data;

	return (
		<>
			<DropdownMenuItem onClick={onChangeLaunchOptions}>
				{tc("projects:menuitem:change launch options")}
			</DropdownMenuItem>
			{unityPath && (
				<DropdownMenuItem onClick={() => setUnityPath.mutate(null)}>
					{tc("projects:menuitem:forget unity path")}
				</DropdownMenuItem>
			)}
			<DropdownMenuItem onClick={openProjectFolder}>
				{tc("projects:menuitem:open directory")}
			</DropdownMenuItem>
			<DropdownMenuItem onClick={onCopyProject}>
				{tc("projects:menuitem:copy project")}
			</DropdownMenuItem>
			<DropdownMenuItem onClick={onBackup}>
				{tc("projects:menuitem:backup")}
			</DropdownMenuItem>
			<DropdownMenuItem
				onClick={removeProject}
				className={"text-destructive focus:text-destructive"}
			>
				{tc("projects:remove project")}
			</DropdownMenuItem>
		</>
	);
}

function ProjectButton({
	projectPath,
	unityVersion,
	unityRevision,
}: {
	projectPath: string;
	unityVersion: string | null;
	unityRevision: string | null;
}) {
	const onChangeLaunchOptions = async () => {
		const initialArgs = await commands.projectGetCustomUnityArgs(projectPath);
		const defaultArgs = await commands.environmentGetDefaultUnityArguments();
		const settings = await openSingleDialog(LaunchSettings, {
			initialValue: initialArgs,
			defaultUnityArgs: defaultArgs,
		});
		if (settings === false) return;
		await commands.projectSetCustomUnityArgs(projectPath, settings);
	};

	return (
		<DropdownMenu>
			<div className={"flex divide-x"}>
				<OpenUnityButton
					projectPath={projectPath}
					unityVersion={unityVersion}
					unityRevision={unityRevision}
					className={"rounded-r-none pl-4 pr-3"}
				/>
				<DropdownMenuTrigger asChild className={"rounded-l-none pl-2 pr-2"}>
					<Button>
						<ChevronDown className={"w-4 h-4"} />
					</Button>
				</DropdownMenuTrigger>
			</div>
			<DropdownMenuContent>
				<DropdownMenuContentBody
					projectPath={projectPath}
					removeProject={() => {
						void openSingleDialog(RemoveProjectDialog, {
							project: {
								path: projectPath,
								is_exists: true,
							},
						});
					}}
					onChangeLaunchOptions={onChangeLaunchOptions}
				/>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
