"use client"

import {
	Button,
	ButtonGroup,
	Card,
	Checkbox,
	IconButton,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Option,
	Select,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React, {Suspense, useMemo} from "react";
import {ArrowLeftIcon, ArrowPathIcon, ChevronDownIcon,} from "@heroicons/react/24/solid";
import {MinusCircleIcon, PlusCircleIcon,} from "@heroicons/react/24/outline";
import {HNavBar, VStack} from "@/components/layout";
import {useSearchParams} from "next/navigation";
import {SearchBox} from "@/components/SearchBox";
import {useQueries} from "@tanstack/react-query";
import {
	environmentPackages,
	projectDetails,
	TauriBasePackageInfo,
	TauriPackage,
	TauriProjectDetails,
	TauriVersion
} from "@/lib/bindings";
import {compareUnityVersion, compareVersion, toVersionString} from "@/lib/version";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
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

	const [packagesResult, detailsResult] = useQueries({
		queries: [
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

	const packageRows = useMemo(() => {
		const packages = packagesResult.status == 'success' ? packagesResult.data : [];
		const details = detailsResult.status == 'success' ? detailsResult.data : null;
		// TODO: visible sources
		return combinePackagesAndProjectDetails(packages, details, null);
	}, [packagesResult, detailsResult]);

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
	};

	return (
		<VStack className={"m-4"}>
			<ProjectViewHeader className={"flex-shrink-0"} projectName={projectName}/>
			<Card className={"flex-shrink-0 p-2 flex flex-row"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden">
					located at: <code className={"bg-gray-200 p-0.5"}>{projectPath}</code>
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
							<IconButton variant={"text"} onClick={onRefresh} className={"flex-shrink-0"}>
								<ArrowPathIcon className={"w-5 h-5"}/>
							</IconButton>
						</Tooltip>

						<SearchBox className={"w-max flex-grow"}/>

						<Menu dismiss={{itemPress: false}}>
							<MenuHandler>
								<Button className={"flex-shrink-0 p-3"}>Select Repositories</Button>
							</MenuHandler>
							<MenuList>
								<MenuItem className="p-0">
									<label className={"flex cursor-pointer items-center gap-2 p-2"}>
										<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
															className="hover:before:content-none"/>
										Official
									</label>
								</MenuItem>
								<MenuItem className="p-0">
									<label className={"flex cursor-pointer items-center gap-2 p-2"}>
										<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
															className="hover:before:content-none"/>
										Curated
									</label>
								</MenuItem>
								<MenuItem className="p-0">
									<label className={"flex cursor-pointer items-center gap-2 p-2"}>
										<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
															className="hover:before:content-none"/>
										anatawa12
									</label>
								</MenuItem>
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
							{packageRows.map((row) => (<PackageRow pkg={row} key={row.id}/>))}
							</tbody>
						</table>
					</Card>
				</Card>
			</main>
		</VStack>
	);
}

interface PackageRowInfo {
	id: string;
	infoSource: TauriVersion;
	displayName: string;
	unityCompatible: Map<string, TauriBasePackageInfo>;
	unityIncompatible: Map<string, TauriBasePackageInfo>;
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
	// null: user local package
	visibleSources: (string | null)[] | null,
): PackageRowInfo[] {
	const visibleSourcesSet = visibleSources ? new Set(visibleSources) : null;
	const packagesTable = new Map<string, PackageRowInfo>();

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

	function getRowInfo(pkg: TauriBasePackageInfo): PackageRowInfo {
		let packageRowInfo = packagesTable.get(pkg.name);
		if (packageRowInfo == null) {
			packagesTable.set(pkg.name, packageRowInfo = {
				id: pkg.name,
				displayName: pkg.display_name ?? pkg.name,
				infoSource: pkg.version,
				unityCompatible: new Map(),
				unityIncompatible: new Map(),
				sources: new Set(),
				installed: null,
			});
		}
		return packageRowInfo;
	}

	const yankedVersions = new Set<`${string}:${string}`>();

	for (const pkg of packages) {
		// TODO: process include Pre-releases
		if (pkg.version.pre) continue;

		if (pkg.is_yanked) {
			yankedVersions.add(`${pkg.name}:${toVersionString(pkg.version)}`);
			continue;
		}

		// check the repository is visible
		if (visibleSourcesSet) {
			if (pkg.source === "LocalUser") {
				if (!visibleSourcesSet.has(null)) continue;
			} else if ('Remote' in pkg.source) {
				if (!visibleSourcesSet.has(pkg.source.Remote.id)) continue;
			}
		}

		const packageRowInfo = getRowInfo(pkg);

		if (compareVersion(pkg.version, packageRowInfo.infoSource) > 0) {
			// use display name from the latest version
			packageRowInfo.infoSource = pkg.version;
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
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

	if (project) {
		for (const [_, pkg] of project.installed_packages) {
			const packageRowInfo = getRowInfo(pkg);

			// if installed, use the installed version to get the display name
			packageRowInfo.displayName = pkg.display_name ?? pkg.name;
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

type PackageInfo = {
	installed: string | null;
	versions: string[];
	displayName: string;
	id: string;
	source: string;
};

function PackageRow({pkg}: { pkg: PackageRowInfo }) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const versionNames = [...pkg.unityCompatible.keys()];
	const latestVersion = versionNames[0];

	let installedInfo;
	if (pkg.installed) {
		const version = toVersionString(pkg.installed.version);
		if (pkg.installed.yanked) {
			installedInfo = `${version} (yanked)`
		} else {
			installedInfo = version;
		}
	} else {
		installedInfo = "Not Installed";
	}

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
				>
					{versionNames.map(v => <Option key={v} value={v}>{v}</Option>)}
				</Select>
			</td>
			<td className={noGrowCellClass}>
				<Typography className="font-normal">
					{latestVersion}
				</Typography>
			</td>
			<td className={`${noGrowCellClass} max-w-32 overflow-hidden`}>
				{
					pkg.sources.size > 1 ? (
						<Tooltip content={[...pkg.sources].join(", ")}>
							<Typography className="font-normal">
								Multiple Sources
							</Typography>
						</Tooltip>
					) : (
						<Typography className="font-normal">
							{[...pkg.sources][0]}
						</Typography>
					)
				}
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					{
						pkg.installed ? (
							<Tooltip content={"Remove Package"}>
								<IconButton variant={'text'}><MinusCircleIcon
									className={"size-5 text-red-700"}/></IconButton>
							</Tooltip>
						) : (
							<Tooltip content={"Add Package"}>
								<IconButton variant={'text'}><PlusCircleIcon
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

			<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
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
