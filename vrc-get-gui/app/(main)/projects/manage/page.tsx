"use client"

import {Button} from "@/components/ui/button";
import {Card} from "@/components/ui/card";
import {Dialog, DialogContent} from "@/components/ui/dialog";
import {DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,} from "@/components/ui/dropdown-menu"
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import {Tooltip, TooltipContent, TooltipTrigger} from "@/components/ui/tooltip";
import React, {Suspense, useCallback, useMemo, useState} from "react";
import {ArrowLeft, ChevronDown} from "lucide-react";
import {HNavBar, VStack} from "@/components/layout";
import {useRouter, useSearchParams} from "next/navigation";
import {useQueries, useQuery, UseQueryResult} from "@tanstack/react-query";
import {
	environmentPackages,
	environmentRefetchPackages,
	environmentRepositoriesInfo,
	environmentUnityVersions,
	projectDetails,
	projectGetUnityPath,
	projectResolve,
	projectSetUnityPath,
	TauriProjectDetails,
	TauriUnityVersions,
	utilOpen
} from "@/lib/bindings";
import {useOpenUnity} from "@/lib/use-open-unity";
import {toastSuccess, toastThrownError} from "@/lib/toast";
import {useRemoveProjectModal} from "@/lib/remove-project";
import {tc} from "@/lib/i18n";
import {nameFromPath} from "@/lib/os";
import {useBackupProjectModal} from "@/lib/backup-project";
import {useUnity2022Migration, useUnity2022PatchMigration} from "./unity-migration";
import {LaunchSettings} from "./launch-settings";
import {PackageListCard} from "./package-list-card";
import {usePackageChangeDialog} from "./use-package-change";
import {combinePackagesAndProjectDetails, VRCSDK_PACKAGES} from "./collect-package-row-info";
import {PageContextProvider} from "./page-context";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
}

function PageBody() {
	const searchParams = useSearchParams();
	const router = useRouter();

	const projectRemoveModal = useRemoveProjectModal({onRemoved: () => router.back()});
	const backupProjectModal = useBackupProjectModal();

	const projectPath = searchParams.get("projectPath") ?? "";
	const projectName = nameFromPath(projectPath);

	// repositoriesInfo: list of repositories and their visibility
	// packagesResult: list of packages
	// detailsResult: project details including installed packages
	// unityVersionsResult: list of unity versions installed
	const [
		repositoriesInfo,
		packagesResult,
		detailsResult,
		unityVersionsResult,
	] = useQueries({
		queries: [
			{
				queryKey: ["environmentRepositoriesInfo"],
				queryFn: environmentRepositoriesInfo,
				refetchOnWindowFocus: false,
			},
			{
				queryKey: ["environmentPackages"],
				queryFn: environmentPackages,
				refetchOnWindowFocus: false,
			},
			{
				queryKey: ["projectDetails", projectPath],
				queryFn: () => projectDetails(projectPath),
				refetchOnWindowFocus: false,
			},
			{
				queryKey: ["environmentUnityVersions"],
				queryFn: () => environmentUnityVersions(),
			},
		]
	});

	const [manualRefetching, setManualRefething] = useState<boolean>(false);

	const packageRowsData = useMemo(() => {
		const packages = packagesResult.data ?? [];
		const details = detailsResult.data ?? null;
		const hiddenRepositories = repositoriesInfo.data?.hidden_user_repositories ?? [];
		const hideUserPackages = repositoriesInfo.data?.hide_local_user_packages ?? false;
		const definedRepositories = repositoriesInfo.data?.user_repositories ?? [];
		const showPrereleasePackages = repositoriesInfo.data?.show_prerelease_packages ?? false;
		return combinePackagesAndProjectDetails(packages, details, hiddenRepositories, hideUserPackages, definedRepositories, showPrereleasePackages);
	}, [repositoriesInfo.data, packagesResult.data, detailsResult.data]);

	const onRefresh = useCallback(async () => {
		try {
			setManualRefething(true);
			await environmentRefetchPackages();
			packagesResult.refetch();
			detailsResult.refetch();
			repositoriesInfo.refetch();
			unityVersionsResult.refetch();
		} finally {
			setManualRefething(false);
		}
	}, [detailsResult, packagesResult, repositoriesInfo, unityVersionsResult]);

	const onRefreshProject = useCallback(() => {
		detailsResult.refetch();
	}, [detailsResult])

	const packageChangeDialog = usePackageChangeDialog({
		projectPath,
		onRefreshProject,
		packageRowsData,
		existingPackages: detailsResult.data?.installed_packages
	});

	const unity2022Migration = useUnity2022Migration({
		projectPath,
		refresh: onRefresh
	});

	const unity2022PatchMigration = useUnity2022PatchMigration({
		projectPath,
		refresh: onRefresh
	});

	const onRefreshRepositories = useCallback(() => {
		repositoriesInfo.refetch();
	}, [repositoriesInfo])

	const onRemoveProject = useCallback(() => {
		projectRemoveModal.startRemove({
			path: projectPath,
			name: projectName,
			is_exists: true,
		})
	}, [projectName, projectPath, projectRemoveModal]);

	const onBackupProject = useCallback(() => {
		backupProjectModal.startBackup({
			path: projectPath,
			name: projectName,
		})
	}, [backupProjectModal, projectName, projectPath]);

	const onResolveRequest = useCallback(() => {
		packageChangeDialog.createChanges({type: "resolve"}, projectResolve(projectPath))
	}, [packageChangeDialog, projectPath])

	const isLoading = packagesResult.isFetching || detailsResult.isFetching || repositoriesInfo.isFetching || unityVersionsResult.isLoading || packageChangeDialog.installingPackage || manualRefetching;

	console.log(`rerender: isloading: ${isLoading}`);

	function checkIfMigrationTo2022Recommended(data: TauriProjectDetails) {
		if (data.unity == null) return false;
		// migrate if the project is using 2019 and has vrcsdk
		if (data.unity[0] != 2019) return false;
		return data.installed_packages.some(([id, _]) => VRCSDK_PACKAGES.includes(id));
	}

	function checkIf2022PatchMigrationRecommended(data: TauriProjectDetails, unityData: TauriUnityVersions) {
		if (!data.installed_packages.some(([id, _]) => VRCSDK_PACKAGES.includes(id))) return false;

		if (data.unity == null) return false;
		if (data.unity[0] != 2022) return false;
		// unity patch is 2022.
		return data.unity_str != unityData.recommended_version;
	}

	const isResolveRecommended = detailsResult?.data?.should_resolve;
	const isMigrationTo2022Recommended = detailsResult.status == 'success' && checkIfMigrationTo2022Recommended(detailsResult.data);
	const is2022PatchMigrationRecommended = detailsResult.status == 'success' && unityVersionsResult.status == 'success'
		&& checkIf2022PatchMigrationRecommended(detailsResult.data, unityVersionsResult.data);

	const pageContext = useMemo(() => ({isLoading}), [isLoading]);

	return (
		<PageContextProvider value={pageContext}>
			<VStack>
				<ProjectViewHeader
					className={"flex-shrink-0"}
					projectName={projectName}
					projectPath={projectPath}
					unityVersion={detailsResult.data?.unity_str ?? null}
					unityRevision={detailsResult.data?.unity_revision ?? null}
					onRemove={onRemoveProject}
					onBackup={onBackupProject}
				/>
				<Card className={"flex-shrink-0 p-2 flex flex-row flex-wrap items-center"}>
					<p className="cursor-pointer py-1.5 font-bold flex-grow flex-shrink overflow-hidden basis-52">
						{tc("projects:manage:project location",
							{path: projectPath},
							{
								components: {
									path: <span className={"p-0.5 font-path whitespace-pre bg-secondary text-secondary-foreground"}/>
								}
							})}
					</p>
					<div className={"flex-grow-0 flex-shrink-0 w-2"}></div>
					<div className="flex-grow-0 flex-shrink-0 flex flex-row items-center">
						<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
							{tc("projects:manage:unity version")}
						</p>
						<div className={"flex-grow-0 flex-shrink-0"}>
							<UnityVersionSelector
								disabled={isLoading}
								detailsResult={detailsResult}
								unityVersions={unityVersionsResult.data}
							/>
						</div>
					</div>
				</Card>
				{isResolveRecommended &&
					<SuggestResolveProjectCard disabled={isLoading}
																		 onResolveRequested={onResolveRequest}/>
				}
				{isMigrationTo2022Recommended &&
					<SuggestMigrateTo2022Card disabled={isLoading}
																		onMigrateRequested={unity2022Migration.request}/>}
				{is2022PatchMigrationRecommended &&
					<Suggest2022PatchMigrationCard disabled={isLoading}
																				 onMigrateRequested={unity2022PatchMigration.request}/>}
				<main className="flex-shrink overflow-hidden flex w-full">
					<PackageListCard
						projectPath={projectPath}
						createChanges={packageChangeDialog.createChanges}
						packageRowsData={packageRowsData}
						repositoriesInfo={repositoriesInfo.data}
						onRefresh={onRefresh}
						onRefreshRepositories={onRefreshRepositories}
					/>
				</main>
				{packageChangeDialog.dialog}
				{unity2022Migration.dialog}
				{unity2022PatchMigration.dialog}
				{projectRemoveModal.dialog}
				{backupProjectModal.dialog}
			</VStack>
		</PageContextProvider>
	);
}

function UnityVersionSelector(
	{
		disabled,
		detailsResult,
		unityVersions,
	}: {
		disabled?: boolean,
		detailsResult: UseQueryResult<TauriProjectDetails>,
		unityVersions?: TauriUnityVersions,
	}
) {
	const unityVersionNames = useMemo(() => {
		if (unityVersions == null) return null
		const versionNames = [...new Set<string>(unityVersions.unity_paths.map(([, path]) => path))];
		versionNames.sort();
		return versionNames;
	}, [unityVersions]);

	const onChange = useCallback(async (version: string) => {
		const detailsData = detailsResult.data;
		if (detailsData == null) return;
		const hasVRCSDK = detailsData.installed_packages.some(([id, _]) => VRCSDK_PACKAGES.includes(id))
		// TODO: show dialog to change unity version
		toastSuccess(`trying to change to ${version}`)
	}, []);

	return (
		<Select disabled={disabled} value={detailsResult.data?.unity_str ?? undefined} onValueChange={onChange}>
			<SelectTrigger>
				<SelectValue placeholder={
					detailsResult.status == 'success' ?
						(detailsResult.data.unity_str ?? "unknown") :
						<span className={"text-primary"}>Loading...</span>
				} className="border-primary/10"/>
			</SelectTrigger>
			<SelectContent>
				<SelectGroup>
					{
						unityVersionNames == null
							? <SelectLabel>Loading...</SelectLabel>
							: unityVersionNames.map(v => <SelectItem key={v} value={v}>{v}</SelectItem>)
					}
				</SelectGroup>
			</SelectContent>
		</Select>
	)
}

function SuggestResolveProjectCard(
	{
		disabled,
		onResolveRequested,
	}: {
		disabled?: boolean;
		onResolveRequested: () => void;
	}
) {
	return (
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p
				className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest resolve")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"}></div>
			<Button variant={"ghost-destructive"} onClick={onResolveRequested} disabled={disabled}>
				{tc("projects:manage:button:resolve")}
			</Button>
		</Card>
	)
}

function SuggestMigrateTo2022Card(
	{
		disabled,
		onMigrateRequested,
	}: {
		disabled?: boolean;
		onMigrateRequested: () => void;
	}
) {
	return (
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p
				className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest unity migration")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"}></div>
			<Button variant={"ghost-destructive"} onClick={onMigrateRequested} disabled={disabled}>
				{tc("projects:manage:button:unity migrate")}
			</Button>
		</Card>
	)
}

function Suggest2022PatchMigrationCard(
	{
		disabled,
		onMigrateRequested,
	}: {
		disabled?: boolean;
		onMigrateRequested: () => void;
	}
) {
	return (
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p
				className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest unity patch migration")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"}></div>
			<Button variant={"ghost-destructive"} onClick={onMigrateRequested} disabled={disabled}>
				{tc("projects:manage:button:unity migrate")}
			</Button>
		</Card>
	)
}

function ProjectViewHeader({
														 className,
														 projectName,
														 projectPath,
														 unityVersion,
														 unityRevision,
														 onRemove,
														 onBackup
													 }: {
	className?: string,
	projectName: string,
	projectPath: string
	unityVersion: string | null,
	unityRevision: string | null,
	onRemove?: () => void,
	onBackup?: () => void,
}) {
	const openUnity = useOpenUnity();
	const [openLaunchOptions, setOpenLaunchOptions] = useState(false);

	const onChangeLaunchOptions = () => setOpenLaunchOptions(true);
	const closeChangeLaunchOptions = () => setOpenLaunchOptions(false);

	return (
		<HNavBar className={className}>
			<Tooltip>
				<TooltipTrigger asChild>
					<Button variant={"ghost"} size={"icon"} onClick={() => history.back()}>
						<ArrowLeft className={"w-5 h-5"}/>
					</Button>
				</TooltipTrigger>
				<TooltipContent>{tc("projects:manage:tooltip:back to projects")}</TooltipContent>
			</Tooltip>

			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 whitespace-pre">
				{projectName}
			</p>

			<div className="relative flex gap-2 w-max flex-grow">
			</div>

			<DropdownMenu>
				<div className={"flex divide-x"}>
					<Button onClick={() => openUnity.openUnity(projectPath, unityVersion, unityRevision)}
									className={"rounded-r-none pl-4 pr-3"}>{tc("projects:button:open unity")}</Button>
					<DropdownMenuTrigger asChild className={"rounded-l-none pl-2 pr-2"}>
						<Button>
							<ChevronDown className={"w-4 h-4"}/>
						</Button>
					</DropdownMenuTrigger>
				</div>
				<DropdownMenuContent>
					<DropdownMenuContentBody
						projectPath={projectPath}
						onRemove={onRemove}
						onBackup={onBackup}
						onChangeLaunchOptions={onChangeLaunchOptions}
					/>
				</DropdownMenuContent>
			</DropdownMenu>
			{openUnity.dialog}
			<Dialog open={openLaunchOptions}>
				<DialogContent>
					<LaunchSettings projectPath={projectPath} close={closeChangeLaunchOptions}/>
				</DialogContent>
			</Dialog>
		</HNavBar>
	);
}

function DropdownMenuContentBody(
	{
		projectPath,
		onRemove,
		onBackup,
		onChangeLaunchOptions,
	}: {
		projectPath: string,
		onRemove?: () => void,
		onBackup?: () => void,
		onChangeLaunchOptions?: () => void,
	}
) {
	const openProjectFolder = () => utilOpen(projectPath, "ErrorIfNotExists");
	const forgetUnity = async () => {
		try {
			await projectSetUnityPath(projectPath, null)
			toastSuccess(tc("projects:toast:forgot unity path"))
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	}
	const unityPathQuery = useQuery({
		queryFn: () => projectGetUnityPath(projectPath),
		queryKey: ["projectGetUnityPath", projectPath],
		refetchOnWindowFocus: false,
	});

	const unityPath = unityPathQuery.data;

	return (
		<>
			<DropdownMenuItem onClick={onChangeLaunchOptions}>
				{tc("projects:menuitem:change launch options")}
			</DropdownMenuItem>
			{unityPath &&
				<DropdownMenuItem onClick={forgetUnity}>{tc("projects:menuitem:forget unity path")}</DropdownMenuItem>}
			<DropdownMenuItem onClick={openProjectFolder}>{tc("projects:menuitem:open directory")}</DropdownMenuItem>
			<DropdownMenuItem onClick={onBackup}>{tc("projects:menuitem:backup")}</DropdownMenuItem>
			<DropdownMenuItem onClick={onRemove} className={"bg-destructive text-destructive-foreground"}>
				{tc("projects:remove project")}
			</DropdownMenuItem>
		</>
	);
}
