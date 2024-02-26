"use client"

import {
	Button,
	ButtonGroup,
	Card,
	Checkbox,
	Dialog,
	DialogBody,
	DialogFooter,
	DialogHeader,
	IconButton,
	List,
	ListItem,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Option,
	Select,
	Spinner,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React, {Suspense, useMemo, useState} from "react";
import {ArrowLeftIcon, ArrowPathIcon, ChevronDownIcon,} from "@heroicons/react/24/solid";
import {MinusCircleIcon, PlusCircleIcon,} from "@heroicons/react/24/outline";
import {HNavBar, VStack} from "@/components/layout";
import {useSearchParams} from "next/navigation";
import {SearchBox} from "@/components/SearchBox";
import {useQueries} from "@tanstack/react-query";
import {
	environmentHideRepository,
	environmentPackages,
	environmentRepositoriesInfo,
	environmentSetHideLocalUserPackages,
	environmentShowRepository,
	projectApplyPendingChanges,
	projectDetails,
	projectInstallPackage,
	projectRemovePackage,
	TauriBasePackageInfo,
	TauriPackage,
	TauriPendingProjectChanges,
	TauriProjectDetails,
	TauriUserRepository,
	TauriVersion
} from "@/lib/bindings";
import {compareUnityVersion, compareVersion, toVersionString} from "@/lib/version";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
}

type InstallStatus = {
	status: "normal";
} | {
	status: "creatingChanges";
} | {
	status: "promptingChanges";
	changes: TauriPendingProjectChanges;
} | {
	status: "applyingChanges";
}

function PageBody() {
	const searchParams = useSearchParams();

	const projectPath = searchParams.get("projectPath") ?? "";
	const projectName = nameFromPath(projectPath);

	function nameFromPath(path: string): string {
		let indexOfSlash = path.lastIndexOf("/");
		let indexOfBackSlash = path.lastIndexOf("\\");
		let indexOfSeparator = Math.max(indexOfSlash, indexOfBackSlash);
		if (indexOfSeparator == -1) return path;
		return path.substring(indexOfSeparator + 1);
	}

	const [repositoriesInfo, packagesResult, detailsResult] = useQueries({
		queries: [
			{
				queryKey: ["environmentRepositoriesInfo"],
				queryFn: environmentRepositoriesInfo,
			},
			{
				queryKey: ["environmentPackages"],
				queryFn: environmentPackages,
			},
			{
				queryKey: ["projectDetails", projectPath],
				queryFn: () => projectDetails(projectPath),
			},
		]
	});

	const [installStatus, setInstallStatus] = useState<InstallStatus>({status: "normal"});
	const [search, setSearch] = useState("");

	const packageRowsData = useMemo(() => {
		const packages = packagesResult.status == 'success' ? packagesResult.data : [];
		const details = detailsResult.status == 'success' ? detailsResult.data : null;
		const hiddenRepositories = repositoriesInfo.status == 'success' ? repositoriesInfo.data.hidden_user_repositories : [];
		const hideUserPackages = repositoriesInfo.status == 'success' ? repositoriesInfo.data.hide_local_user_packages : false;
		const definedRepositories = repositoriesInfo.status == 'success' ? repositoriesInfo.data.user_repositories : [];
		return combinePackagesAndProjectDetails(packages, details, hiddenRepositories, hideUserPackages, definedRepositories);
	}, [repositoriesInfo, packagesResult, detailsResult]);

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
		"Package",
		"Installed",
		"Latest",
		"Source",
		"", // actions
	];

	const unityVersions = [
		'2019.4.31f1',
		'2020.3.14f1',
		'2021.1.5f1',
	];

	const onRefresh = () => {
		packagesResult.refetch();
		detailsResult.refetch();
		repositoriesInfo.refetch();
	};

	const onInstallRequested = async (pkg: TauriPackage) => {
		try {
			setInstallStatus({status: "creatingChanges"});
			console.log("install", pkg.name, pkg.version);
			const changes = await projectInstallPackage(projectPath, pkg.env_version, pkg.index);
			setInstallStatus({status: "promptingChanges", changes});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
		}
	}

	const onRemoveRequested = async (pkgId: string) => {
		try {
			setInstallStatus({status: "creatingChanges"});
			console.log("remove", pkgId);
			const changes = await projectRemovePackage(projectPath, pkgId);
			setInstallStatus({status: "promptingChanges", changes});
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
		}
	}

	const applyChanges = async (changes: TauriPendingProjectChanges) => {
		try {
			setInstallStatus({status: "applyingChanges"});
			await projectApplyPendingChanges(projectPath, changes.changes_version);
			setInstallStatus({status: "normal"});
			detailsResult.refetch();
		} catch (e) {
			console.error(e);
			setInstallStatus({status: "normal"});
		}
	}

	const installingPackage = installStatus.status != "normal";
	const isLoading = packagesResult.isFetching || detailsResult.isFetching || repositoriesInfo.isFetching || installingPackage;

	return (
		<VStack className={"m-4"}>
			<ProjectViewHeader className={"flex-shrink-0"} projectName={projectName}/>
			<Card className={"flex-shrink-0 p-2 flex flex-row"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden">
					located at: <code className={"bg-gray-200 p-0.5 whitespace-pre"}>{projectPath}</code>
				</Typography>
				<div className={"flex-grow flex-shrink-0 w-2"}></div>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
					Unity Version:
				</Typography>
				<div className={"flex-grow-0 flex-shrink-0"}>
					<Select variant={'outlined'} value={"2019.4.31f1"} labelProps={{className: "hidden"}}
									className="border-blue-gray-200">
						{unityVersions.map(v => <Option key={v} value={v}>{v}</Option>)}
					</Select>
				</div>
			</Card>
			<main className="flex-shrink overflow-hidden flex">
				<Card className="w-full p-2 gap-2 flex-grow flex-shrink flex">
					<div className={"flex flex-shrink-0 flex-grow-0 flex-row gap-2"}>
						<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
							Manage Packages
						</Typography>

						<Tooltip content="Reflesh Packages">
							<IconButton variant={"text"} onClick={onRefresh} className={"flex-shrink-0"} disabled={isLoading}>
								{isLoading ? <Spinner className="w-5 h-5"/> : <ArrowPathIcon className={"w-5 h-5"}/>}
							</IconButton>
						</Tooltip>

						<SearchBox className={"w-max flex-grow"} value={search} onChange={e => setSearch(e.target.value)}/>

						<Menu dismiss={{itemPress: false}}>
							<MenuHandler>
								<Button className={"flex-shrink-0 p-3"}>Select Repositories</Button>
							</MenuHandler>
							<MenuList className={"max-h-96 w-64"}>
								<RepositoryMenuItem
									hiddenUserRepositories={hiddenUserRepositories}
									repositoryName={"Official"}
									repositoryId={"com.vrchat.repos.official"}
									refetch={() => repositoriesInfo.refetch()}
								/>
								<RepositoryMenuItem
									hiddenUserRepositories={hiddenUserRepositories}
									repositoryName={"Curated"}
									repositoryId={"com.vrchat.repos.curated"}
									refetch={() => repositoriesInfo.refetch()}
								/>
								<UserLocalRepositoryMenuItem
									hideUserLocalPackages={repositoriesInfo.status == 'success' ? repositoriesInfo.data.hide_local_user_packages : false}
									refetch={() => repositoriesInfo.refetch()}
								/>
								<hr className="my-3"/>
								{
									repositoriesInfo.status == 'success' ? repositoriesInfo.data.user_repositories.map(repository => (
										<RepositoryMenuItem
											hiddenUserRepositories={hiddenUserRepositories}
											repositoryName={repository.display_name}
											repositoryId={repository.id}
											refetch={() => repositoriesInfo.refetch()}
											key={repository.id}
										/>
									)) : null
								}
							</MenuList>
						</Menu>
					</div>
					<Card className="w-full overflow-x-auto overflow-y-scroll">
						<table className="relative table-auto text-left">
							<thead>
							<tr>
								{TABLE_HEAD.map((head, index) => (
									<th key={index}
											className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
										<Typography variant="small" className="font-normal leading-none">{head}</Typography>
									</th>
								))}
							</tr>
							</thead>
							<tbody>
							{packageRows.map((row) => (
								<PackageRow pkg={row} key={row.id}
														locked={isLoading}
														onInstallRequested={onInstallRequested}
														onRemoveRequested={onRemoveRequested}/>
							))}
							</tbody>
						</table>
					</Card>
				</Card>
				{
					installStatus.status === "promptingChanges" ? (
						<ProjectChangesDialog changes={installStatus.changes}
																	cancel={() => setInstallStatus({status: "normal"})}
																	apply={() => applyChanges(installStatus.changes)}
						/>
					) : null
				}
			</main>
		</VStack>
	);
}

function ProjectChangesDialog(
	{
		changes,
		cancel,
		apply,
	}: {
		changes: TauriPendingProjectChanges,
		cancel: () => void,
		apply: () => void,
	}) {
	const versionConflicts = changes.conflicts.filter(([_, c]) => c.packages.length > 0);
	const unityConflicts = changes.conflicts.filter(([_, c]) => c.unity_conflict);

	return (
		<Dialog open handler={() => {
		}}>
			<DialogHeader>Apply Changes</DialogHeader>
			<DialogBody>
				<Typography className={"text-gray-900"}>
					You're applying the following changes to the project
				</Typography>
				<List>
					{changes.package_changes.map(([pkgId, pkgChange]) => {
						if ('InstallNew' in pkgChange) {
							return <ListItem key={pkgId}>
								Install {pkgChange.InstallNew.display_name ?? pkgChange.InstallNew.name} version {toVersionString(pkgChange.InstallNew.version)}
							</ListItem>
						} else {
							switch (pkgChange.Remove) {
								case "Requested":
									return <ListItem key={pkgId}>Remove {pkgId} since you requested.</ListItem>
								case "Legacy":
									return <ListItem key={pkgId}>Remove {pkgId} since it's a legacy package.</ListItem>
								case "Unused":
									return <ListItem key={pkgId}>Remove {pkgId} since it's unused.</ListItem>
							}
						}
					})}
				</List>
				{
					versionConflicts.length > 0 ? (
						<>
							<Typography className={"text-red-700"}>
								There are version conflicts
							</Typography>
							<List>
								{versionConflicts.map(([pkgId, conflict]) => (
									<ListItem key={pkgId}>
										{pkgId} conflicts with {conflict.packages.map(p => p).join(", ")}
									</ListItem>
								))}
							</List>
						</>
					) : null
				}
				{
					unityConflicts.length > 0 ? (
						<>
							<Typography className={"text-red-700"}>
								There are unity version conflicts
							</Typography>
							<List>
								{unityConflicts.map(([pkgId, _]) => (
									<ListItem key={pkgId}>
										{pkgId} does not support your unity version
									</ListItem>
								))}
							</List>
						</>
					) : null
				}
				{
					changes.remove_legacy_files.length > 0 || changes.remove_legacy_folders.length > 0 ? (
						<>
							<Typography className={"text-red-700"}>
								The following legacy files and folders will be removed
							</Typography>
							<List>
								{changes.remove_legacy_files.map(f => (
									<ListItem key={f}>
										{f}
									</ListItem>
								))}
								{changes.remove_legacy_folders.map(f => (
									<ListItem key={f}>
										{f}
									</ListItem>
								))}
							</List>
						</>
					) : null
				}
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">Cancel</Button>
				<Button onClick={apply} color={"red"}>Apply</Button>
			</DialogFooter>
		</Dialog>
	);
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
		<MenuItem className="p-0">
			<label className={"flex cursor-pointer items-center gap-2 p-2 whitespace-normal"}>
				<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
									checked={selected}
									onChange={onChange}
									className="hover:before:content-none"/>
				{repositoryName}
			</label>
		</MenuItem>
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
		<MenuItem className="p-0">
			<label className={"flex cursor-pointer items-center gap-2 p-2"}>
				<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
									checked={selected}
									onChange={onChange}
									className="hover:before:content-none"/>
				User Local
			</label>
		</MenuItem>
	)
}

interface PackageRowInfo {
	id: string;
	infoSource: TauriVersion;
	displayName: string;
	aliases: string[];
	unityCompatible: Map<string, TauriPackage>;
	unityIncompatible: Map<string, TauriPackage>;
	sources: Set<string>;
	installed: null | {
		version: TauriVersion;
		yanked: boolean;
	};
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
	const packagesPerRepository = new Map<string, TauriPackage[]>();
	const userPackages: TauriPackage[] = [];

	for (const pkg of packages) {
		// TODO: process include Pre-releases
		if (pkg.version.pre) continue;

		if (pkg.is_yanked) {
			yankedVersions.add(`${pkg.name}:${toVersionString(pkg.version)}`);
			continue;
		}

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
				installed: null,
			});
		}
		return packageRowInfo;
	};

	function addPackage(pkg: TauriPackage) {
		const packageRowInfo = getRowInfo(pkg);

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
		}
	}

	// sort versions
	for (let value of packagesTable.values()) {
		value.unityCompatible = new Map([...value.unityCompatible].sort((a, b) => -compareVersion(a[1].version, b[1].version)));
		value.unityIncompatible = new Map([...value.unityIncompatible].sort((a, b) => -compareVersion(a[1].version, b[1].version)));
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

function PackageRow(
	{
		pkg,
		locked,
		onInstallRequested,
		onRemoveRequested,
	}: {
		pkg: PackageRowInfo;
		locked: boolean;
		onInstallRequested: (pkg: TauriPackage) => void;
		onRemoveRequested: (pkgId: string) => void;
	}) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const versionNames = [...pkg.unityCompatible.keys()];
	const latestVersion: string | undefined = versionNames[0];

	const notInstalled = "Not Installed";
	let installedInfo: string;
	if (pkg.installed) {
		const version = toVersionString(pkg.installed.version);
		if (pkg.installed.yanked) {
			installedInfo = `${version} (yanked)`
		} else {
			installedInfo = version;
		}
	} else {
		installedInfo = notInstalled;
	}

	const onChange = (version: string | undefined) => {
		if (!version) return;
		const pkgVersion = pkg.unityCompatible.get(version);
		if (!pkgVersion) return;
		onInstallRequested(pkgVersion);
	}

	const installLatest = () => {
		if (!latestVersion) return;
		const latest = pkg.unityCompatible.get(latestVersion);
		if (!latest) return;
		onInstallRequested(latest);
	}

	const remove = () => {
		onRemoveRequested(pkg.id);
	};

	return (
		<tr className="even:bg-blue-gray-50/50">
			<td className={`${cellClass} overflow-hidden max-w-80 overflow-ellipsis`}>
				<div className="flex flex-col">
					<Typography className="font-normal">
						{pkg.displayName}
					</Typography>
					<Typography className="font-normal opacity-50 text-sm">
						{pkg.id}
					</Typography>
				</div>
			</td>
			<td className={noGrowCellClass}>
				{/* This is broken: popup is not shown out of the card */}
				{/* TODO: show incompatible versions */}
				{/* TODO: install with selecting version */}
				<Select value={installedInfo}
								labelProps={{className: "hidden"}}
								menuProps={{className: "z-20"}}
								className={`border-blue-gray-200 ${pkg.installed?.yanked ? "text-red-700" : ""}`}
								onChange={onChange}
								selected={() => <>{installedInfo}</>}
								disabled={locked}
				>
					{versionNames.map(v => <Option key={v} value={v}>{v}</Option>)}
					<Option value={notInstalled} hidden>{notInstalled}</Option>
				</Select>
			</td>
			<td className={noGrowCellClass}>
				{
					latestVersion ? <Typography className="font-normal">{latestVersion}</Typography>
						: <Typography className="font-normal text-blue-gray-400">none</Typography>
				}
			</td>
			<td className={`${noGrowCellClass} max-w-32 overflow-hidden`}>
				{
					pkg.sources.size == 0 ? (
						<Typography className="font-normal text-blue-gray-400">
							none
						</Typography>
					) : pkg.sources.size == 1 ? (
						<Typography className="font-normal">
							{[...pkg.sources][0]}
						</Typography>
					) : (
						<Tooltip content={[...pkg.sources].join(", ")}>
							<Typography className="font-normal">
								Multiple Sources
							</Typography>
						</Tooltip>
					)
				}
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					{
						pkg.installed ? (
							<Tooltip content={"Remove Package"}>
								<IconButton variant={'text'} disabled={locked} onClick={remove}><MinusCircleIcon
									className={"size-5 text-red-700"}/></IconButton>
							</Tooltip>
						) : (
							<Tooltip content={"Add Package"}>
								<IconButton variant={'text'} disabled={locked && !!latestVersion}
														onClick={installLatest}><PlusCircleIcon
									className={"size-5 text-gray-800"}/></IconButton>
							</Tooltip>
						)
					}
				</div>
			</td>
		</tr>
	);
}

function ProjectViewHeader({className, projectName}: { className?: string, projectName: string }) {
	return (
		<HNavBar className={className}>
			<Tooltip content="Back to projects">
				<IconButton variant={"text"} onClick={() => history.back()}>
					<ArrowLeftIcon className={"w-5 h-5"}/>
				</IconButton>
			</Tooltip>

			<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 whitespace-pre">
				{projectName}
			</Typography>

			<div className="relative flex gap-2 w-max flex-grow">
			</div>

			<Menu>
				<ButtonGroup>
					<Button className={"pl-4 pr-3"}>Open Unity</Button>
					<MenuHandler className={"pl-2 pr-2"}>
						<Button>
							<ChevronDownIcon className={"w-4 h-4"}/>
						</Button>
					</MenuHandler>
				</ButtonGroup>
				<MenuList>
					<MenuItem>Open Project Folder</MenuItem>
					<MenuItem>Make Backup</MenuItem>
					<MenuItem className={"bg-red-700 text-white"}>Remove Project</MenuItem>
				</MenuList>
			</Menu>
		</HNavBar>
	);
}
