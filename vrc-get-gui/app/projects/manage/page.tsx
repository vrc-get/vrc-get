"use client"

import {Button} from "@/components/ui/button";
import {Card, CardHeader} from "@/components/ui/card";
import {Checkbox} from "@/components/ui/checkbox";
import {Dialog, DialogContent, DialogTitle} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {Tooltip, TooltipContent, TooltipTrigger} from "@/components/ui/tooltip";
import React, {Fragment, memo, Suspense, useCallback, useMemo, useState} from "react";
import {ArrowLeftIcon, ArrowPathIcon, ChevronDownIcon, EllipsisHorizontalIcon,} from "@heroicons/react/24/solid";
import {ArrowUpCircleIcon, MinusCircleIcon, PlusCircleIcon,} from "@heroicons/react/24/outline";
import {HNavBar, VStack} from "@/components/layout";
import {useRouter, useSearchParams} from "next/navigation";
import {SearchBox} from "@/components/SearchBox";
import {useQueries} from "@tanstack/react-query";
import {
	environmentHideRepository,
	environmentPackages,
	environmentRefetchPackages,
	environmentRepositoriesInfo,
	environmentSetHideLocalUserPackages,
	environmentShowRepository,
	environmentUnityVersions,
	projectApplyPendingChanges,
	projectDetails,
	projectInstallMultiplePackage,
	projectInstallPackage,
	projectRemovePackages,
	projectResolve,
	projectUpgradeMultiplePackage,
	TauriBasePackageInfo,
	TauriPackage,
	TauriPackageChange,
	TauriPendingProjectChanges,
	TauriProjectDetails,
	TauriUnityVersions,
	TauriUserRepository,
	TauriVersion,
	utilOpen
} from "@/lib/bindings";
import {compareUnityVersion, compareVersion, toVersionString} from "@/lib/version";
import {VGOption, VGSelect} from "@/components/select";
import {useOpenUnity} from "@/lib/use-open-unity";
import {shellOpen} from "@/lib/shellOpen";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {useRemoveProjectModal} from "@/lib/remove-project";
import {tc, tt} from "@/lib/i18n";
import {nameFromPath} from "@/lib/os";
import {useBackupProjectModal} from "@/lib/backup-project";
import {useUnity2022Migration, useUnity2022PatchMigration} from "@/app/projects/manage/unity-migration";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
}

type RequestedOperation = {
	type: "install";
	pkg: TauriPackage;
} | {
	type: "upgradeAll";
} | {
	type: "remove";
	pkgId: string;
} | {
	type: "bulkInstalled"
} | {
	type: "bulkRemoved"
}

type InstallStatus = {
	status: "normal";
} | {
	status: "creatingChanges";
} | {
	status: "promptingChanges";
	changes: TauriPendingProjectChanges;
	requested: RequestedOperation;
} | {
	status: "applyingChanges";
}

type BulkUpdateMode = 'install' | 'upgradeOrRemove' | 'remove' | 'upgrade' | 'any';
type PackageBulkUpdateMode = 'install' | 'upgradeOrRemove' | 'remove';

function updateModeFromPackageModes(map: PackageBulkUpdateMode[]): BulkUpdateMode {
	const asSet = new Set(map);

	if (asSet.size == 0) {
		return 'any';
	}
	if (asSet.size == 1) {
		return [...asSet][0];
	}
	if (asSet.size == 2) {
		if (asSet.has('remove') && asSet.has('upgradeOrRemove')) {
			return 'remove';
		}
	}

	return "any";
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
	const [repositoriesInfo, packagesResult, detailsResult, unityVersionsResult] = useQueries({
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

	const [installStatus, setInstallStatus] = useState<InstallStatus>({status: "normal"});
	const [manualRefetching, setManualRefething] = useState<boolean>(false);
	const [search, setSearch] = useState("");
	const [bulkUpdatePackageIds, setBulkUpdatePackageIds] = useState<[id: string, mode: PackageBulkUpdateMode][]>([]);
	const bulkUpdateMode = useMemo(() => updateModeFromPackageModes(bulkUpdatePackageIds.map(([_, mode]) => mode)), [bulkUpdatePackageIds]);

	const packageRowsData = useMemo(() => {
		const packages = packagesResult.status == 'success' ? packagesResult.data : [];
		const details = detailsResult.status == 'success' ? detailsResult.data : null;
		const hiddenRepositories = repositoriesInfo.status == 'success' ? repositoriesInfo.data.hidden_user_repositories : [];
		const hideUserPackages = repositoriesInfo.status == 'success' ? repositoriesInfo.data.hide_local_user_packages : false;
		const definedRepositories = repositoriesInfo.status == 'success' ? repositoriesInfo.data.user_repositories : [];
		const showPrereleasePackages = repositoriesInfo.status == 'success' ? repositoriesInfo.data.show_prerelease_packages : false;
		return combinePackagesAndProjectDetails(packages, details, hiddenRepositories, hideUserPackages, definedRepositories, showPrereleasePackages);
	}, [
		repositoriesInfo.status, repositoriesInfo.data,
		packagesResult.status, packagesResult.data,
		detailsResult.status, detailsResult.data,
	]);

	const packageRows = useMemo(() => {
		if (search === "") return packageRowsData;
		const searchLower = search.toLowerCase();
		return packageRowsData.filter(row =>
			row.displayName.toLowerCase().includes(searchLower)
			|| row.id.toLowerCase().includes(searchLower)
			|| row.aliases.some(alias => alias.toLowerCase().includes(searchLower)))
	}, [packageRowsData, search]);

	const hiddenUserRepositories = useMemo(() => new Set(repositoriesInfo.status == 'success' ? repositoriesInfo.data.hidden_user_repositories : []), [repositoriesInfo]);

	const TABLE_HEAD = [
		"projects:manage:package",
		"projects:manage:installed",
		"projects:manage:latest",
		"general:source",
	];

	// TODO: get installed unity versions and show them
	const unityVersions: string[] = []

	const onRefresh = async () => {
		setBulkUpdatePackageIds([]);
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
	};

	const onRefreshRepositories = () => {
		repositoriesInfo.refetch();
	}

	const onRefreshProject = () => {
		detailsResult.refetch();
		setBulkUpdatePackageIds([]);
	}

	const onRemoveProject = () => {
		projectRemoveModal.startRemove({
			path: projectPath,
			name: projectName,
			is_exists: true,
		})
	}

	const onBackupProject = () => {
		backupProjectModal.startBackup({
			path: projectPath,
			name: projectName,
		})
	}

	const onInstallRequested = useCallback(async (pkg: TauriPackage) => {
		try {
			setInstallStatus({status: "creatingChanges"});
			console.log("install", pkg.name, pkg.version);
			const changes = await projectInstallPackage(projectPath, pkg.env_version, pkg.index);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "install", pkg}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	}, [projectPath]);

	const onUpgradeAllRequest = async () => {
		try {
			setInstallStatus({status: "creatingChanges"});
			let packages: number[] = [];
			let envVersion: number | undefined = undefined;
			for (let packageRow of packageRows) {
				if (packageRow.latest.status === "upgradable") {
					if (envVersion == null) envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion != packageRow.latest.pkg.env_version) throw new Error("Inconsistent env_version");
					packages.push(packageRow.latest.pkg.index);
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			const changes = await projectUpgradeMultiplePackage(projectPath, envVersion, packages);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "upgradeAll"}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	}

	const onResolveRequest = async () => {
		try {
			setInstallStatus({status: "creatingChanges"});
			const changes = await projectResolve(projectPath);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "upgradeAll"}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	};

	const onRemoveRequested = useCallback(async (pkgId: string) => {
		try {
			setInstallStatus({status: "creatingChanges"});
			console.log("remove", pkgId);
			const changes = await projectRemovePackages(projectPath, [pkgId]);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "remove", pkgId}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	}, [projectPath]);

	const onUpgradeBulkRequested = async () => {
		try {
			setInstallStatus({status: "creatingChanges"});
			let packageIds = new Set(bulkUpdatePackageIds.map(([id, mode]) => id));
			let packages: number[] = [];
			let envVersion: number | undefined = undefined;
			for (let packageRow of packageRows) {
				if (packageIds.has(packageRow.id)) {
					if (packageRow.latest.status !== "upgradable")
						throw new Error("Package is not upgradable");

					if (envVersion == null) envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion != packageRow.latest.pkg.env_version) throw new Error("Inconsistent env_version");

					packages.push(packageRow.latest.pkg.index);
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			const changes = await projectUpgradeMultiplePackage(projectPath, envVersion, packages);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "upgradeAll"}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	};

	const onInstallBulkRequested = async () => {
		try {
			setInstallStatus({status: "creatingChanges"});
			let packageIds = new Set(bulkUpdatePackageIds.map(([id, mode]) => id));
			let packages: number[] = [];
			let envVersion: number | undefined = undefined;
			for (let packageRow of packageRows) {
				if (packageIds.has(packageRow.id)) {
					if (packageRow.latest.status !== "contains")
						throw new Error("Package is not installable");

					if (envVersion == null) envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion != packageRow.latest.pkg.env_version) throw new Error("Inconsistent env_version");

					packages.push(packageRow.latest.pkg.index);
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			const changes = await projectInstallMultiplePackage(projectPath, envVersion, packages);
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "bulkInstalled"}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	};

	const onRemoveBulkRequested = async () => {
		try {
			setInstallStatus({status: "creatingChanges"});
			const changes = await projectRemovePackages(projectPath, bulkUpdatePackageIds.map(([id, mode]) => id));
			setInstallStatus({status: "promptingChanges", changes, requested: {type: "bulkRemoved"}});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	};

	const addBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		const possibleUpdate: PackageBulkUpdateMode | 'nothing' = bulkUpdateModeForPackage(row);

		if (possibleUpdate == 'nothing') return;
		setBulkUpdatePackageIds(prev => {
			if (prev.some(([id, _]) => id === row.id)) return prev;
			return [...prev, [row.id, possibleUpdate]];
		});
	}, []);

	const removeBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		setBulkUpdatePackageIds(prev => prev.filter(([id, _]) => id !== row.id));
	}, [setBulkUpdatePackageIds]);

	const applyChanges = async (
		{
			changes,
			requested,
		}: {
			changes: TauriPendingProjectChanges,
			requested: RequestedOperation,
		}) => {
		try {
			setInstallStatus({status: "applyingChanges"});
			await projectApplyPendingChanges(projectPath, changes.changes_version);
			setInstallStatus({status: "normal"});
			onRefreshProject();

			switch (requested.type) {
				case "install":
					toastSuccess(tt("projects:manage:toast:package installed",
						{name: requested.pkg.display_name ?? requested.pkg.name, version: toVersionString(requested.pkg.version)}));
					break;
				case "remove":
					toastSuccess(tt("projects:manage:toast:package removed", {name: requested.pkgId}));
					break;
				case "upgradeAll":
					toastSuccess(tt("projects:manage:toast:all packages upgraded"));
					break;
				case "bulkInstalled":
					toastSuccess(tt("projects:manage:toast:selected packages installed"));
					break;
				case "bulkRemoved":
					toastSuccess(tt("projects:manage:toast:selected packages removed"));
					break;
				default:
					let _: never = requested;
			}
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
			toastThrownError(e);
		}
	}

	const unity2022Migration = useUnity2022Migration({
		projectPath,
		unityVersions: unityVersionsResult.data,
		refresh: onRefresh
	});
	const unity2022PatchMigration = useUnity2022PatchMigration({
		projectPath,
		unityVersions: unityVersionsResult.data,
		refresh: onRefresh
	});

	const installingPackage = installStatus.status != "normal";
	const isLoading = packagesResult.isFetching || detailsResult.isFetching || repositoriesInfo.isFetching || unityVersionsResult.isLoading || installingPackage || manualRefetching;

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

	let dialogForState: React.ReactNode = null;

	switch (installStatus.status) {
		case "promptingChanges":
			dialogForState = <ProjectChangesDialog
				packages={packageRowsData}
				changes={installStatus.changes}
				cancel={() => setInstallStatus({status: "normal"})}
				apply={() => applyChanges(installStatus)}
			/>;
			break;
	}

	return (
		<VStack className={"m-4"}>
			<ProjectViewHeader
				className={"flex-shrink-0"}
				projectName={projectName}
				projectPath={projectPath}
				unityVersion={detailsResult.data?.unity_str ?? null}
				unityRevision={detailsResult.data?.unity_revision ?? null}
				unityVersions={unityVersionsResult?.data}
				onRemove={onRemoveProject}
				onBackup={onBackupProject}
			/>
			<Card className={"flex-shrink-0 p-2 flex flex-row flex-wrap"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow flex-shrink overflow-hidden basis-52">
					{tc("projects:manage:project location",
						{path: projectPath},
						{
							components: {path: <span className={"p-0.5 font-path whitespace-pre bg-secondary text-secondary-foreground"}/>}
						})}
				</p>
				<div className={"flex-grow-0 flex-shrink-0 w-2"}></div>
				<div className="flex-grow-0 flex-shrink-0 flex flex-row">
					<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
						{tc("projects:manage:unity version")}
					</p>
					<div className={"flex-grow-0 flex-shrink-0"}>
						<VGSelect value={detailsResult.status == 'success' ? (detailsResult.data.unity_str ?? "unknown") :
							<span className={"text-primary"}>Loading...</span>}
											className="border-primary/10">
							{/*unityVersions.map(v => <VGOption key={v} value={v}>{v}</VGOption>)*/}
							<VGOption value={""}>{tc("general:not implemented")}</VGOption>
						</VGSelect>
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
			<main className="flex-shrink overflow-hidden flex">
				<Card className="flex-grow flex-shrink flex shadow-none">
          <CardHeader className="w-full p-2 gap-2">
            <div className={"flex flex-wrap flex-shrink-0 flex-grow-0 flex-row gap-2"}>
              <p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
                {tc("projects:manage:manage packages")}
              </p>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant={"ghost"} onClick={onRefresh} className={"flex-shrink-0 -px-4 -py-2 min-w-10 min-h-10"} disabled={isLoading}>
                    {isLoading ? <ArrowPathIcon className="w-5 h-5 animate-spin"/> : <ArrowPathIcon className={"w-5 h-5"}/>}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{tc("projects:manage:tooltip:refresh packages")}</TooltipContent>
              </Tooltip>

              <SearchBox className={"w-max flex-grow"} value={search} onChange={e => setSearch(e.target.value)}/>

              {packageRows.some(row => row.latest.status === "upgradable") &&
                <Button className={"flex-shrink-0"}
                        onClick={onUpgradeAllRequest}
                        disabled={isLoading}
                        variant={"success"}>
                  {tc("projects:manage:button:upgrade all")}
                </Button>}

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant={"ghost"} className={'flex-shrink-0 -px-4 -py-2 min-w-10 min-h-10'}>
                    <EllipsisHorizontalIcon className={"size-5"}/>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                  <DropdownMenuItem className={"p-3"}
                            onClick={onResolveRequest}
                            disabled={isLoading}>
                    {tc("projects:manage:button:reinstall all")}</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button className={"flex-shrink-0 p-3"}>{tc("projects:manage:button:select repositories")}</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className={"max-h-96 w-64"}>
                  <RepositoryMenuItem
                    hiddenUserRepositories={hiddenUserRepositories}
                    repositoryName={tt("vpm repositories:source:official")}
                    repositoryId={"com.vrchat.repos.official"}
                    refetch={onRefreshRepositories}
                  />
                  <RepositoryMenuItem
                    hiddenUserRepositories={hiddenUserRepositories}
                    repositoryName={tt("vpm repositories:source:curated")}
                    repositoryId={"com.vrchat.repos.curated"}
                    refetch={onRefreshRepositories}
                  />
                  <UserLocalRepositoryMenuItem
                    hideUserLocalPackages={repositoriesInfo.status == 'success' ? repositoriesInfo.data.hide_local_user_packages : false}
                    refetch={onRefreshRepositories}
                  />
                  <hr className="my-3"/>
                  {
                    repositoriesInfo.status == 'success' ? repositoriesInfo.data.user_repositories.map(repository => (
                      <RepositoryMenuItem
                        hiddenUserRepositories={hiddenUserRepositories}
                        repositoryName={repository.display_name}
                        repositoryId={repository.id}
                        refetch={onRefreshRepositories}
                        key={repository.id}
                      />
                    )) : null
                  }
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
            <BulkUpdateCard
              disabled={isLoading} bulkUpdateMode={bulkUpdateMode}
              bulkUpgradeAll={onUpgradeBulkRequested}
              bulkRemoveAll={onRemoveBulkRequested}
              bulkInstallAll={onInstallBulkRequested}
              cancel={() => setBulkUpdatePackageIds([])}
            />
            <Card className="w-full overflow-x-auto overflow-y-scroll">
              <CardHeader>
              <table className="relative table-auto text-left">
                <thead>
                <tr>
                  <th className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground`}>
                  </th>
                  {TABLE_HEAD.map((head, index) => (
                    <th key={index}
                        className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5`}>
                      <small className="font-normal leading-none">{tc(head)}</small>
                    </th>
                  ))}
                  <th className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5`}/>
                </tr>
                </thead>
                <tbody>
                {packageRows.map((row) => (
                  <PackageRow pkg={row} key={row.id}
                              locked={isLoading}
                              onInstallRequested={onInstallRequested}
                              onRemoveRequested={onRemoveRequested}
                              bulkUpdateSelected={bulkUpdatePackageIds.some(([id, _]) => id === row.id)}
                              bulkUpdateAvailable={canBulkUpdate(bulkUpdateMode, bulkUpdateModeForPackage(row))}
                              addBulkUpdatePackage={addBulkUpdatePackage}
                              removeBulkUpdatePackage={removeBulkUpdatePackage}
                  />))}
                </tbody>
              </table>
              </CardHeader>
            </Card>
          </CardHeader>
				</Card>
				{dialogForState}
				{unity2022Migration.dialog}
				{unity2022PatchMigration.dialog}
				{projectRemoveModal.dialog}
				{backupProjectModal.dialog}
			</main>
		</VStack>
	);
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

function BulkUpdateCard(
	{
		disabled,
		bulkUpdateMode,
		bulkUpgradeAll,
		bulkRemoveAll,
		bulkInstallAll,
		cancel,
	}: {
		disabled: boolean;
		bulkUpdateMode: BulkUpdateMode;
		bulkUpgradeAll?: () => void;
		bulkRemoveAll?: () => void;
		bulkInstallAll?: () => void;
		cancel?: () => void;
	}
) {
	if (bulkUpdateMode == 'any') return null;

	const canInstall = bulkUpdateMode == 'install';
	const canUpgrade = bulkUpdateMode == 'upgrade' || bulkUpdateMode == 'upgradeOrRemove';
	const canRemove = bulkUpdateMode == 'remove' || bulkUpdateMode == 'upgradeOrRemove';

	return (
		<Card className={"flex-shrink-0 p-2 flex flex-row gap-2 bg-secondary text-secondary-foreground flex-wrap"}>
			{canInstall && <Button disabled={disabled} onClick={bulkInstallAll}>
				{tc("projects:manage:button:install selected")}
			</Button>}
			{canUpgrade && <Button disabled={disabled} onClick={bulkUpgradeAll} variant={"success"}>
				{tc("projects:manage:button:upgrade selected")}
			</Button>}
			{canRemove && <Button disabled={disabled} onClick={bulkRemoveAll} variant={"destructive"}>
				{tc("projects:manage:button:uninstall selected")}
			</Button>}
			<Button disabled={disabled} onClick={cancel}>
				{tc("projects:manage:button:clear selection")}
			</Button>
		</Card>
	)
}

function ProjectChangesDialog(
	{
		changes,
		packages,
		cancel,
		apply,
	}: {
		changes: TauriPendingProjectChanges,
		packages: PackageRowInfo[],
		cancel: () => void,
		apply: () => void,
	}) {
	const versionConflicts = changes.conflicts.filter(([_, c]) => c.packages.length > 0);
	const unityConflicts = changes.conflicts.filter(([_, c]) => c.unity_conflict);

	const getPackageDisplayName = useMemo(() => {
		const packagesById = new Map(packages.map(p => [p.id, p]));
		return (pkgId: string) => packagesById.get(pkgId)?.displayName ?? pkgId;
	}, [packages]);

	const TypographyItem = ({children}: { children: React.ReactNode }) => (
		<div className={"p-3"}><p className={"font-normal"}>{children}</p></div>
	);

	const packageChangesSorted = changes.package_changes.sort(comparePackageChange);

	return (
		<Dialog open>
      <DialogContent className={"whitespace-normal"}>
        <DialogTitle>{tc("projects:manage:button:apply changes")}</DialogTitle>
        <div className={"overflow-y-auto max-h-[50vh]"}>
          <p>
            {tc("projects:manage:dialog:confirm changes description")}
          </p>
          <div className={"flex flex-col gap-1 p-2"}>
            {packageChangesSorted.map(([pkgId, pkgChange]) => {
              if ('InstallNew' in pkgChange) {
                let changelogUrlTmp = pkgChange.InstallNew.changelog_url;
                if (changelogUrlTmp != null && !changelogUrlTmp.startsWith("http") && !changelogUrlTmp.startsWith("https"))
                  changelogUrlTmp = null;
                const changelogUrl = changelogUrlTmp;
                return <div key={pkgId} className={"p-3"}>
                  <p className={"font-normal"}>{tc("projects:manage:dialog:install package", {
                    name: pkgChange.InstallNew.display_name ?? pkgChange.InstallNew.name,
                    version: toVersionString(pkgChange.InstallNew.version),
                  })}</p>
                  {changelogUrl != null &&
                    <Button className={"ml-1 px-2"} size={"sm"}
                            onClick={() => shellOpen(changelogUrl)}>{tc("projects:manage:button:see changelog")}</Button>}
                </div>
              } else {
                const name = getPackageDisplayName(pkgId);
                switch (pkgChange.Remove) {
                  case "Requested":
                    return <TypographyItem key={pkgId}>
                      {tc("projects:manage:dialog:uninstall package as requested", {name})}
                    </TypographyItem>
                  case "Legacy":
                    return <TypographyItem key={pkgId}>
                      {tc("projects:manage:dialog:uninstall package as legacy", {name})}
                    </TypographyItem>
                  case "Unused":
                    return <TypographyItem key={pkgId}>
                      {tc("projects:manage:dialog:uninstall package as unused", {name})}
                    </TypographyItem>
                }
              }
            })}
          </div>
          {
            versionConflicts.length > 0 ? (
              <>
                <p className={"text-destructive"}>
                  {tc("projects:manage:dialog:package version conflicts", {count: versionConflicts.length})}
                </p>
                <div className={"flex flex-col gap-1 p-2"}>
                  {versionConflicts.map(([pkgId, conflict]) => {
                    return (
                      <TypographyItem key={pkgId}>
                        {tc("projects:manage:dialog:conflicts with", {
                          pkg: getPackageDisplayName(pkgId),
                          other: conflict.packages.map(p => getPackageDisplayName(p)).join(", ")
                        })}
                      </TypographyItem>
                    );
                  })}
                </div>
              </>
            ) : null
          }
          {
            unityConflicts.length > 0 ? (
              <>
                <p className={"text-destructive"}>
                  {tc("projects:manage:dialog:unity version conflicts", {count: unityConflicts.length})}
                </p>
                <div className={"flex flex-col gap-1 p-2"}>
                  {unityConflicts.map(([pkgId, _]) => (
                    <TypographyItem key={pkgId}>
                      {tc("projects:manage:dialog:package not supported your unity", {pkg: getPackageDisplayName(pkgId)})}
                    </TypographyItem>
                  ))}
                </div>
              </>
            ) : null
          }
          {
            changes.remove_legacy_files.length > 0 || changes.remove_legacy_folders.length > 0 ? (
              <>
                <p className={"text-destructive"}>
                  {tc("projects:manage:dialog:files and directories are removed as legacy")}
                </p>
                <div className={"flex flex-col gap-1 p-2"}>
                  {changes.remove_legacy_files.map(f => (
                    <TypographyItem key={f}>
                      {f}
                    </TypographyItem>
                  ))}
                  {changes.remove_legacy_folders.map(f => (
                    <TypographyItem key={f}>
                      {f}
                    </TypographyItem>
                  ))}
                </div>
              </>
            ) : null
          }
        </div>
        <div className={"ml-auto"}>
          <Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
          <Button onClick={apply} variant={"destructive"}>{tc("projects:manage:button:apply")}</Button>
        </div>
      </DialogContent>
		</Dialog>
	);
}

function comparePackageChange([aName, aChange]: [string, TauriPackageChange], [bName, bChange]: [string, TauriPackageChange]): number {
	const aType = packageChangesType(aChange);
	const bType = packageChangesType(bChange);
	if (aType !== bType) return aType - bType;
	return aName.localeCompare(bName);
}

function packageChangesType(pkgChange: TauriPackageChange): 0 | 1 | 2 | 3 {
	if ('InstallNew' in pkgChange) return 0;
	switch (pkgChange.Remove) {
		case "Requested":
			return 1;
		case "Legacy":
			return 2;
		case "Unused":
			return 3;
	}
}

function RepositoryMenuItem(
	{
		hiddenUserRepositories,
		repositoryName,
		repositoryId,
		refetch,
	}: {
		hiddenUserRepositories: Set<string>,
		repositoryName: string,
		repositoryId: string,
		refetch: () => void,
	}
) {
	const selected = !hiddenUserRepositories.has(repositoryId);
	const onChange = () => {
		if (selected) {
			environmentHideRepository(repositoryId).then(refetch);
		} else {
			environmentShowRepository(repositoryId).then(refetch);
		}
	};

	return (
		<DropdownMenuItem className="p-0" onSelect={ (e) => { e.preventDefault() } }>
			<label className={"flex cursor-pointer items-center gap-2 p-2 whitespace-normal"}>
				<Checkbox checked={selected}
									onCheckedChange={onChange}
									className="hover:before:content-none"/>
				{repositoryName}
			</label>
		</DropdownMenuItem>
	)
}

function UserLocalRepositoryMenuItem(
	{
		hideUserLocalPackages,
		refetch,
	}: {
		hideUserLocalPackages: boolean,
		refetch: () => void,
	}
) {
	const selected = !hideUserLocalPackages;
	const onChange = () => {
		if (selected) {
			environmentSetHideLocalUserPackages(true).then(refetch);
		} else {
			environmentSetHideLocalUserPackages(false).then(refetch);
		}
	};

	return (
		<DropdownMenuItem className="p-0" onSelect={ (e) => { e.preventDefault() } }>
			<label className={"flex cursor-pointer items-center gap-2 p-2"}>
				<Checkbox checked={selected}
									onCheckedChange={onChange}
									className="hover:before:content-none"/>
				{tc("vpm repositories:source:local")}
			</label>
		</DropdownMenuItem>
	)
}

type PackageLatestInfo = { status: "none" } | { status: "contains", pkg: TauriPackage } | {
	status: "upgradable",
	pkg: TauriPackage
};

interface PackageRowInfo {
	id: string;
	infoSource: TauriVersion;
	displayName: string;
	aliases: string[];
	unityCompatible: Map<string, TauriPackage>;
	unityIncompatible: Map<string, TauriPackage>;
	sources: Set<string>;
	isThereSource: boolean; // this will be true even if all sources are hidden
	installed: null | {
		version: TauriVersion;
		yanked: boolean;
	};
	latest: PackageLatestInfo;
}

const VRCSDK_PACKAGES = [
	"com.vrchat.avatars",
	"com.vrchat.worlds",
	"com.vrchat.base"
];

function combinePackagesAndProjectDetails(
	packages: TauriPackage[],
	project: TauriProjectDetails | null,
	hiddenRepositories?: string[] | null,
	hideLocalUserPackages?: boolean,
	definedRepositories: TauriUserRepository[] = [],
	showPrereleasePackages: boolean = false,
): PackageRowInfo[] {
	const hiddenRepositoriesSet = new Set(hiddenRepositories ?? []);

	function isUnityCompatible(pkg: TauriPackage, unityVersion: [number, number] | null) {
		if (unityVersion == null) return true;
		if (pkg.unity == null) return true;

		// vrcsdk exceptions for unity version
		if (VRCSDK_PACKAGES.includes(pkg.name)) {
			if (pkg.version.major === 3 && pkg.version.minor <= 4) {
				return unityVersion[0] === 2019;
			}
		} else if (pkg.name === "com.vrchat.core.vpm-resolver") {
			if (pkg.version.major === 0 && pkg.version.minor === 1 && pkg.version.patch <= 26) {
				return unityVersion[0] === 2019;
			}
		}

		return compareUnityVersion(pkg.unity, unityVersion) <= 0;
	}

	const yankedVersions = new Set<`${string}:${string}`>();
	const knownPackages = new Set<string>();
	const packagesPerRepository = new Map<string, TauriPackage[]>();
	const userPackages: TauriPackage[] = [];

	for (const pkg of packages) {
		if (!showPrereleasePackages && pkg.version.pre) continue;

		if (pkg.is_yanked) {
			yankedVersions.add(`${pkg.name}:${toVersionString(pkg.version)}`);
			continue;
		}

		knownPackages.add(pkg.name);

		let packages: TauriPackage[]
		// check the repository is visible
		if (pkg.source === "LocalUser") {
			if (hideLocalUserPackages) continue
			packages = userPackages;
		} else if ('Remote' in pkg.source) {
			if (hiddenRepositoriesSet.has(pkg.source.Remote.id)) continue;

			packages = packagesPerRepository.get(pkg.source.Remote.id) ?? [];
			packagesPerRepository.set(pkg.source.Remote.id, packages);
		} else {
			let never: never = pkg.source;
			throw new Error("unreachable");
		}

		packages.push(pkg);

	}

	const packagesTable = new Map<string, PackageRowInfo>();

	const getRowInfo = (pkg: TauriBasePackageInfo): PackageRowInfo => {
		let packageRowInfo = packagesTable.get(pkg.name);
		if (packageRowInfo == null) {
			packagesTable.set(pkg.name, packageRowInfo = {
				id: pkg.name,
				displayName: pkg.display_name ?? pkg.name,
				aliases: pkg.aliases,
				infoSource: pkg.version,
				unityCompatible: new Map(),
				unityIncompatible: new Map(),
				sources: new Set(),
				isThereSource: false,
				installed: null,
				latest: {status: "none"},
			});
		}
		return packageRowInfo;
	};

	function addPackage(pkg: TauriPackage) {
		const packageRowInfo = getRowInfo(pkg);
		packageRowInfo.isThereSource = true;

		if (compareVersion(pkg.version, packageRowInfo.infoSource) > 0) {
			// use display name from the latest version
			packageRowInfo.infoSource = pkg.version;
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
			packageRowInfo.aliases = pkg.aliases;
		}

		if (project == null || isUnityCompatible(pkg, project.unity)) {
			packageRowInfo.unityCompatible.set(toVersionString(pkg.version), pkg);
		} else {
			packageRowInfo.unityIncompatible.set(toVersionString(pkg.version), pkg);
		}

		if (pkg.source === "LocalUser") {
			packageRowInfo.sources.add("User");
		} else if ('Remote' in pkg.source) {
			packageRowInfo.sources.add(pkg.source.Remote.display_name);
		}
	}

	// predefined repositories
	packagesPerRepository.get("com.vrchat.repos.official")?.forEach(addPackage);
	packagesPerRepository.get("com.vrchat.repos.curated")?.forEach(addPackage);
	userPackages.forEach(addPackage);
	packagesPerRepository.delete("com.vrchat.repos.official");
	packagesPerRepository.delete("com.vrchat.repos.curated");

	// for repositories
	for (let definedRepository of definedRepositories) {
		packagesPerRepository.get(definedRepository.id)?.forEach(addPackage);
		packagesPerRepository.delete(definedRepository.id);
	}

	// in case of repository is not defined
	for (let packages of packagesPerRepository.values()) {
		packages.forEach(addPackage);
	}

	// sort versions
	for (let value of packagesTable.values()) {
		value.unityCompatible = new Map([...value.unityCompatible].sort((a, b) => -compareVersion(a[1].version, b[1].version)));
		value.unityIncompatible = new Map([...value.unityIncompatible].sort((a, b) => -compareVersion(a[1].version, b[1].version)));
	}

	// set latest info
	for (let value of packagesTable.values()) {
		const latestPackage = value.unityCompatible.values().next().value;
		if (latestPackage) {
			value.latest = {status: "contains", pkg: latestPackage};
		}
	}

	// set installed info
	if (project) {
		for (const [_, pkg] of project.installed_packages) {
			const packageRowInfo = getRowInfo(pkg);

			// if installed, use the installed version to get the display name
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
			packageRowInfo.aliases = [...pkg.aliases, ...packageRowInfo.aliases];
			packageRowInfo.installed = {
				version: pkg.version,
				yanked: pkg.is_yanked || yankedVersions.has(`${pkg.name}:${toVersionString(pkg.version)}`),
			};
			packageRowInfo.isThereSource = knownPackages.has(pkg.name);

			// if we have the latest version, check if it's upgradable
			if (packageRowInfo.latest.status != "none") {
				const compare = compareVersion(pkg.version, packageRowInfo.latest.pkg.version);
				if (compare < 0) {
					packageRowInfo.latest = {status: "upgradable", pkg: packageRowInfo.latest.pkg};
				}
			}
		}
	}

	const isAvatarsSdkInstalled = packagesTable.get("com.vrchat.avatars")?.installed != null;
	const isWorldsSdkInstalled = packagesTable.get("com.vrchat.worlds")?.installed != null;
	if (isAvatarsSdkInstalled != isWorldsSdkInstalled) {
		// if either avatars or worlds sdk is installed, remove the packages for the other SDK.

		// collect dependant packages
		const dependantPackages = new Map<string, Set<string>>();
		for (let pkg of packagesTable.values()) {
			if (pkg.latest.status != "none") {
				for (const dependency of pkg.latest.pkg.vpm_dependencies) {
					if (!dependantPackages.has(dependency)) {
						dependantPackages.set(dependency, new Set());
					}
					dependantPackages.get(dependency)!.add(pkg.id);
				}
			}
		}

		const toRemove = new Set<string>();

		// remove the other SDK
		if (isAvatarsSdkInstalled) {
			toRemove.add("com.vrchat.worlds");
		} else if (isWorldsSdkInstalled) {
			toRemove.add("com.vrchat.avatars");
		}

		// update forAvatars and forWorlds recursively
		while (toRemove.size > 0) {
			const pkgId = [...toRemove].pop()!;
			toRemove.delete(pkgId);

			if (!packagesTable.delete(pkgId)) continue // already removed

			const dependants = dependantPackages.get(pkgId);
			if (dependants != null)
				for (const dependant of dependants)
					toRemove.add(dependant);
		}
	}

	if (project) {
		for (const [_, pkg] of project.installed_packages) {
			for (const legacyPackage of pkg.legacy_packages) {
				packagesTable.delete(legacyPackage);
			}
		}
	}

	const asArray = Array.from(packagesTable.values());

	// put installed first
	asArray.sort((a, b) => {
		if (a.installed && !b.installed) return -1;
		if (!a.installed && b.installed) return 1;
		return 0;
	});

	return asArray;
}

const PackageRow = memo(function PackageRow(
	{
		pkg,
		locked,
		onInstallRequested,
		onRemoveRequested,
		bulkUpdateSelected,
		bulkUpdateAvailable,
		addBulkUpdatePackage,
		removeBulkUpdatePackage,
	}: {
		pkg: PackageRowInfo;
		locked: boolean;
		onInstallRequested: (pkg: TauriPackage) => void;
		onRemoveRequested: (pkgId: string) => void;
		bulkUpdateSelected: boolean;
		bulkUpdateAvailable: boolean;
		addBulkUpdatePackage: (pkg: PackageRowInfo) => void;
		removeBulkUpdatePackage: (pkg: PackageRowInfo) => void;
	}) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const versionNames = [...pkg.unityCompatible.keys()];
	const latestVersion: string | undefined = versionNames[0];
	useCallback((version: string) => {
		if (pkg.installed != null && version === toVersionString(pkg.installed.version)) return;
		const pkgVersion = pkg.unityCompatible.get(version) ?? pkg.unityIncompatible.get(version);
		if (!pkgVersion) return;
		onInstallRequested(pkgVersion);
	}, [onInstallRequested, pkg.installed]);
	const installLatest = () => {
		if (!latestVersion) return;
		const latest = pkg.unityCompatible.get(latestVersion) ?? pkg.unityIncompatible.get(latestVersion);
		if (!latest) return;
		onInstallRequested(latest);
	}

	const remove = () => {
		onRemoveRequested(pkg.id);
	};

	const onClickBulkUpdate = () => {
		if (bulkUpdateSelected) {
			removeBulkUpdatePackage(pkg);
		} else {
			addBulkUpdatePackage(pkg);
		}
	}

	return (
		<tr className="even:bg-secondary/30">
			<td className={`${cellClass} w-1`}>
				<Checkbox checked={bulkUpdateSelected}
									onCheckedChange={onClickBulkUpdate}
									disabled={locked || !bulkUpdateAvailable}
									className="hover:before:content-none"/>
			</td>
			<td className={`${cellClass} overflow-hidden max-w-80 overflow-ellipsis ${pkg.installed ? '' : 'opacity-50'}`}>
				<div className="flex flex-col">
					<p className="font-normal">
						{pkg.displayName}
					</p>
					<p className="font-normal opacity-50 text-sm">
						{pkg.id}
					</p>
				</div>
			</td>
			<td className={noGrowCellClass}>
				<PackageVersionSelector pkg={pkg} onInstallRequested={onInstallRequested} locked={locked}/>
			</td>
			<td className={`${cellClass} min-w-32 w-32`}>
				<PackageLatestInfo info={pkg.latest} locked={locked} onInstallRequested={onInstallRequested}/>
			</td>
			<td className={`${noGrowCellClass} max-w-32 overflow-hidden`}>
				{
					pkg.sources.size == 0 ? (
						pkg.isThereSource ? (
							<p>
								{tc("projects:manage:source not selected")}
							</p>
						) : (
							<p>
								{tc("projects:manage:none")}
							</p>
						)
					) : pkg.sources.size == 1 ? (
            <Tooltip>
              <TooltipTrigger>
                <p className="overflow-hidden overflow-ellipsis">
                  {[...pkg.sources][0]}
                </p>
              </TooltipTrigger>
              <TooltipContent>{[ ...pkg.sources][0] }</TooltipContent>
            </Tooltip>
					) : (
            <Tooltip>
              <TooltipTrigger>
                <p>
                  {tc("projects:manage:multiple sources")}
                </p>
              </TooltipTrigger>
              <TooltipContent>{ [...pkg.sources].join(", ") }</TooltipContent>
            </Tooltip>
					)
				}
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					{
						pkg.installed ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button className={"-px-4 -py-2 min-w-10 min-h-10"} variant={'ghost'} disabled={locked} onClick={remove}><MinusCircleIcon
                    className={"size-5 text-destructive"}/></Button>
                </TooltipTrigger>
                <TooltipContent>{tc("projects:manage:tooltip:remove packages")}</TooltipContent>
              </Tooltip>
						) : (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button className={"-px-4 -py-2 min-w-10 min-h-10"} variant={'ghost'} disabled={locked && !!latestVersion}
                              onClick={installLatest}><PlusCircleIcon
                    className={"size-5 text-secondary-foreground"}/></Button>
                </TooltipTrigger>
                <TooltipContent>{tc("projects:manage:tooltip:add package")}</TooltipContent>
              </Tooltip>
						)
					}
				</div>
			</td>
		</tr>
	);
});

function bulkUpdateModeForPackage(pkg: PackageRowInfo): PackageBulkUpdateMode | 'nothing' {
	if (pkg.installed) {
		if (pkg.latest.status === "upgradable") {
			return "upgradeOrRemove";
		} else {
			return "remove";
		}
	} else {
		if (pkg.latest.status !== "none") {
			return "install";
		} else {
			return "nothing";
		}
	}
}

const PackageVersionSelector = memo(function PackageVersionSelector(
	{
		pkg,
		onInstallRequested,
		locked,
	}: {
		pkg: PackageRowInfo;
		onInstallRequested: (pkg: TauriPackage) => void;
		locked: boolean;
	}
) {
	const onChange = useCallback((version: string) => {
		if (pkg.installed != null && version === toVersionString(pkg.installed.version)) return;
		const pkgVersion = pkg.unityCompatible.get(version) ?? pkg.unityIncompatible.get(version);
		if (!pkgVersion) return;
		onInstallRequested(pkgVersion);
	}, [onInstallRequested, pkg.installed]);

	const versionNames = [...pkg.unityCompatible.keys()];
	const incompatibleNames = [...pkg.unityIncompatible.keys()];

	return (
		<VGSelect value={<PackageInstalledInfo pkg={pkg}/>}
							className={`border-primary/10 ${pkg.installed?.yanked ? "text-destructive" : ""}`}
							onChange={onChange}
							disabled={locked}
		>
			{versionNames.map(v => <VGOption key={v} value={v}>{v}</VGOption>)}
			{(incompatibleNames.length > 0 && versionNames.length > 0) && <hr className="my-2"/>}
			{incompatibleNames.length > 0 &&
				<p className={"text-sm"}>{tc("projects:manage:incompatible packages")}</p>}
			{incompatibleNames.map(v => <VGOption key={v} value={v}>{v}</VGOption>)}
		</VGSelect>
	);
})

function canBulkUpdate(bulkUpdateMode: BulkUpdateMode, possibleUpdate: PackageBulkUpdateMode | 'nothing'): boolean {
	if (possibleUpdate === "nothing") return false;
	if (bulkUpdateMode === "any") return true;
	if (bulkUpdateMode === possibleUpdate) return true;
	if (bulkUpdateMode === "upgradeOrRemove" && possibleUpdate === "remove") return true;
	if (bulkUpdateMode === "upgrade" && possibleUpdate === "upgradeOrRemove") return true;
	if (bulkUpdateMode === "remove" && possibleUpdate === "upgradeOrRemove") return true;
	return false;
}

function PackageInstalledInfo(
	{
		pkg,
	}: {
		pkg: PackageRowInfo,
	}
) {
	if (pkg.installed) {
		const version = toVersionString(pkg.installed.version);
		if (pkg.installed.yanked) {
			return <p className={"text-destructive"}>{version} {tc("projects:manage:yanked")}</p>;
		} else {
			return <p>{version}</p>;
		}
	} else {
		return <p className="text-muted-foreground/70">{tc("projects:manage:none")}</p>;
	}
}

function PackageLatestInfo(
	{
		info,
		locked,
		onInstallRequested,
	}: {
		info: PackageLatestInfo,
		locked: boolean,
		onInstallRequested: (pkg: TauriPackage) => void;
	}
) {
	switch (info.status) {
		case "none":
			return <p className="text-muted-foreground">{tc("projects:manage:none")}</p>;
		case "contains":
			return <p>{toVersionString(info.pkg.version)}</p>;
		case "upgradable":
			return (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant={"outline"}
                    className={"text-left px-2 py-1 w-full h-full font-normal text-base normal-case border-success hover:border-success/70 text-success hover:text-success/70"}
                    disabled={locked}
                    onClick={() => onInstallRequested(info.pkg)}>
              <ArrowUpCircleIcon color={"green"} className={"size-4 inline mr-2"}/>
              {toVersionString(info.pkg.version)}
            </Button>
          </TooltipTrigger>
          <TooltipContent>{tc("projects:manage:tooltip:upgrade package")}</TooltipContent>
        </Tooltip>
			);
		default:
			let _: never = info;
	}
}

function ProjectViewHeader({
														 className,
														 projectName,
														 projectPath,
														 unityVersion,
														 unityRevision,
														 unityVersions,
														 onRemove,
														 onBackup
													 }: {
	className?: string,
	projectName: string,
	projectPath: string
	unityVersion: string | null,
	unityRevision: string | null,
	unityVersions: TauriUnityVersions | undefined,
	onRemove?: () => void,
	onBackup?: () => void,
}) {
	const openUnity = useOpenUnity(unityVersions);
	const openProjectFolder = () => utilOpen(projectPath);

	return (
		<HNavBar className={className}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button className={"-px-4 -py-2 min-w-10 min-h-10"} variant={"ghost"} onClick={() => history.back()}>
            <ArrowLeftIcon className={"w-5 h-5"}/>
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
							<ChevronDownIcon className={"w-4 h-4"}/>
						</Button>
					</DropdownMenuTrigger>
				</div>
				<DropdownMenuContent>
					<DropdownMenuItem onClick={openProjectFolder}>{tc("projects:menuitem:open directory")}</DropdownMenuItem>
					<DropdownMenuItem onClick={onBackup}>{tc("projects:menuitem:backup")}</DropdownMenuItem>
					<DropdownMenuItem onClick={onRemove} className={"bg-destructive text-destructive-foreground"}>{tc("projects:remove project")}</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
			{openUnity.dialog}
		</HNavBar>
	);
}
