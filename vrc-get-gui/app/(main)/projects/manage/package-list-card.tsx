// noinspection ExceptionCaughtLocallyJS

import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
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
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriPackage,
	TauriPendingProjectChanges,
	TauriRepositoriesInfo,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc, tt } from "@/lib/i18n";
import { toastError, toastThrownError } from "@/lib/toast";
import { toVersionString } from "@/lib/version";
import {
	CircleArrowUp,
	CircleMinus,
	CirclePlus,
	Ellipsis,
	RefreshCw,
} from "lucide-react";
import type React from "react";
import { memo, useCallback, useEffect, useMemo, useState } from "react";
import type {
	PackageLatestInfo,
	PackageRowInfo,
} from "./collect-package-row-info";
import {
	ButtonDisabledIfLoading,
	CheckboxDisabledIfLoading,
	usePageContext,
} from "./page-context";

type RequestedOperation =
	| {
			type: "install";
			pkg: TauriPackage;
			hasUnityIncompatibleLatest?: boolean;
	  }
	| {
			type: "upgradeAll";
			hasUnityIncompatibleLatest: boolean;
	  }
	| {
			type: "resolve";
	  }
	| {
			type: "reinstallAll";
	  }
	| {
			type: "remove";
			displayName: string;
	  }
	| {
			type: "bulkInstalled";
			hasUnityIncompatibleLatest: boolean;
	  }
	| {
			type: "bulkRemoved";
	  };

type BulkUpdateMode =
	| "install"
	| "upgradeOrRemove"
	| "remove"
	| "upgrade"
	| "any";
type PackageBulkUpdateMode = "install" | "upgradeOrRemove" | "remove";

function updateModeFromPackageModes(
	map: PackageBulkUpdateMode[],
): BulkUpdateMode {
	const asSet = new Set(map);

	if (asSet.size === 0) {
		return "any";
	}
	if (asSet.size === 1) {
		return [...asSet][0];
	}
	if (asSet.size === 2) {
		if (asSet.has("remove") && asSet.has("upgradeOrRemove")) {
			return "remove";
		}
	}

	return "any";
}

export const PackageListCard = memo(function PackageListCard({
	projectPath,
	createChanges,
	packageRowsData,
	repositoriesInfo,
	onRefresh,
	onRefreshRepositories,
}: {
	projectPath: string;
	createChanges: (
		operation: RequestedOperation,
		create: Promise<TauriPendingProjectChanges>,
	) => void;
	packageRowsData: PackageRowInfo[];
	repositoriesInfo: TauriRepositoriesInfo | undefined;
	onRefresh: () => void;
	onRefreshRepositories: () => void;
}) {
	const [search, setSearch] = useState("");
	const [bulkUpdatePackageIds, setBulkUpdatePackageIds] = useState<
		[id: string, mode: PackageBulkUpdateMode][]
	>([]);
	const bulkUpdateMode = useMemo(
		() =>
			updateModeFromPackageModes(bulkUpdatePackageIds.map(([_, mode]) => mode)),
		[bulkUpdatePackageIds],
	);

	const filteredPackageIds = useMemo(() => {
		if (search === "") return new Set<string>(packageRowsData.map((x) => x.id));
		const searchLower = search.toLowerCase();
		return new Set<string>(
			packageRowsData
				.filter(
					(row) =>
						row.displayName.toLowerCase().includes(searchLower) ||
						row.id.toLowerCase().includes(searchLower) ||
						row.aliases.some((alias) =>
							alias.toLowerCase().includes(searchLower),
						),
				)
				.map((x) => x.id),
		);
	}, [packageRowsData, search]);

	const hiddenUserRepositories = useMemo(
		() => new Set(repositoriesInfo?.hidden_user_repositories ?? []),
		[repositoriesInfo],
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: reset when packageRowsData changes
	useEffect(() => {
		// if packageRowsData is changed, clear bulkUpdatePackageIds
		setBulkUpdatePackageIds([]);
	}, [packageRowsData]);

	const onInstallRequested = useCallback(
		(pkg: TauriPackage, hasUnityIncompatibleLatest?: boolean) => {
			createChanges(
				{
					type: "install",
					pkg,
					hasUnityIncompatibleLatest,
				},
				commands.projectInstallPackages(projectPath, pkg.env_version, [
					pkg.index,
				]),
			);
		},
		[projectPath, createChanges],
	);

	const onUpgradeAllRequest = useCallback(() => {
		try {
			const packages: number[] = [];
			let envVersion: number | undefined = undefined;
			let hasUnityIncompatibleLatest = false;
			for (const packageRow of packageRowsData) {
				if (packageRow.latest.status === "upgradable") {
					if (envVersion == null)
						envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion !== packageRow.latest.pkg.env_version)
						throw new Error("Inconsistent env_version");
					packages.push(packageRow.latest.pkg.index);
					hasUnityIncompatibleLatest ||=
						packageRow.latest.hasUnityIncompatibleLatest;
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			createChanges(
				{
					type: "upgradeAll",
					hasUnityIncompatibleLatest,
				},
				commands.projectInstallPackages(projectPath, envVersion, packages),
			);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	}, [createChanges, projectPath, packageRowsData]);

	const onReinstallRequest = useCallback(
		() =>
			createChanges(
				{ type: "reinstallAll" },
				commands.projectResolve(projectPath),
			),
		[createChanges, projectPath],
	);

	const onRemoveRequested = useCallback(
		async (pkg: PackageRowInfo) =>
			createChanges(
				{ type: "remove", displayName: pkg.displayName },
				commands.projectRemovePackages(projectPath, [pkg.id]),
			),
		[createChanges, projectPath],
	);

	const onUpgradeBulkRequested = useCallback(() => {
		try {
			const packageIds = new Set(bulkUpdatePackageIds.map(([id, _]) => id));
			const packages: number[] = [];
			let envVersion: number | undefined = undefined;
			let hasUnityIncompatibleLatest = false;
			for (const packageRow of packageRowsData) {
				if (packageIds.has(packageRow.id)) {
					if (packageRow.latest.status !== "upgradable")
						throw new Error("Package is not upgradable");

					if (envVersion == null)
						envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion !== packageRow.latest.pkg.env_version)
						throw new Error("Inconsistent env_version");

					packages.push(packageRow.latest.pkg.index);
					hasUnityIncompatibleLatest ||=
						packageRow.latest.hasUnityIncompatibleLatest;
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			createChanges(
				{
					type: "upgradeAll",
					hasUnityIncompatibleLatest,
				},
				commands.projectInstallPackages(projectPath, envVersion, packages),
			);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	}, [bulkUpdatePackageIds, createChanges, packageRowsData, projectPath]);

	const onInstallBulkRequested = useCallback(() => {
		try {
			const packageIds = new Set(bulkUpdatePackageIds.map(([id, _]) => id));
			const packages: number[] = [];
			let envVersion: number | undefined = undefined;
			let hasUnityIncompatibleLatest = false;
			for (const packageRow of packageRowsData) {
				if (packageIds.has(packageRow.id)) {
					if (packageRow.latest.status !== "contains")
						throw new Error("Package is not installable");

					if (envVersion == null)
						envVersion = packageRow.latest.pkg.env_version;
					else if (envVersion !== packageRow.latest.pkg.env_version)
						throw new Error("Inconsistent env_version");

					packages.push(packageRow.latest.pkg.index);
					hasUnityIncompatibleLatest ||=
						packageRow.latest.hasUnityIncompatibleLatest;
				}
			}
			if (envVersion == null) {
				toastError(tt("projects:manage:toast:no upgradable"));
				return;
			}
			createChanges(
				{ type: "bulkInstalled", hasUnityIncompatibleLatest },
				commands.projectInstallPackages(projectPath, envVersion, packages),
			);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	}, [bulkUpdatePackageIds, createChanges, packageRowsData, projectPath]);

	const onRemoveBulkRequested = useCallback(() => {
		createChanges(
			{ type: "bulkRemoved" },
			commands.projectRemovePackages(
				projectPath,
				bulkUpdatePackageIds.map(([id, _]) => id),
			),
		);
	}, [bulkUpdatePackageIds, createChanges, projectPath]);

	const addBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		const possibleUpdate: PackageBulkUpdateMode | "nothing" =
			bulkUpdateModeForPackage(row);

		if (possibleUpdate === "nothing") return;
		setBulkUpdatePackageIds((prev) => {
			if (prev.some(([id, _]) => id === row.id)) return prev;
			return [...prev, [row.id, possibleUpdate]];
		});
	}, []);

	const removeBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		setBulkUpdatePackageIds((prev) => prev.filter(([id, _]) => id !== row.id));
	}, []);

	const dialogForState: React.ReactNode = null;

	const TABLE_HEAD = [
		"projects:manage:package",
		"projects:manage:installed",
		"projects:manage:latest",
		"general:source",
	];

	return (
		<Card className="flex-grow flex-shrink flex shadow-none w-full">
			<CardContent className="w-full p-2 flex flex-col gap-2">
				<ManagePackagesHeading
					packageRowsData={packageRowsData}
					hiddenUserRepositories={hiddenUserRepositories}
					repositoriesInfo={repositoriesInfo}
					onRefresh={onRefresh}
					onRefreshRepositories={onRefreshRepositories}
					onUpgradeAllRequest={onUpgradeAllRequest}
					onReinstallRequest={onReinstallRequest}
					search={search}
					setSearch={setSearch}
				/>
				<BulkUpdateCard
					bulkUpdateMode={bulkUpdateMode}
					bulkUpgradeAll={onUpgradeBulkRequested}
					bulkRemoveAll={onRemoveBulkRequested}
					bulkInstallAll={onInstallBulkRequested}
					cancel={() => setBulkUpdatePackageIds([])}
				/>
				<ScrollableCardTable>
					<thead>
						<tr>
							<th
								className={
									"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground"
								}
							/>
							{TABLE_HEAD.map((head, index) => (
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
							<th
								className={
									"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
								}
							/>
						</tr>
					</thead>
					<tbody>
						{packageRowsData.map((row) => (
							<tr
								className="even:bg-secondary/30"
								hidden={!filteredPackageIds.has(row.id)}
								key={row.id}
							>
								<PackageRow
									pkg={row}
									onInstallRequested={onInstallRequested}
									onRemoveRequested={onRemoveRequested}
									bulkUpdateSelected={bulkUpdatePackageIds.some(
										([id, _]) => id === row.id,
									)}
									bulkUpdateAvailable={canBulkUpdate(
										bulkUpdateMode,
										bulkUpdateModeForPackage(row),
									)}
									addBulkUpdatePackage={addBulkUpdatePackage}
									removeBulkUpdatePackage={removeBulkUpdatePackage}
								/>
							</tr>
						))}
					</tbody>
				</ScrollableCardTable>
			</CardContent>
			{dialogForState}
		</Card>
	);
});

function ManagePackagesHeading({
	packageRowsData,
	hiddenUserRepositories,
	repositoriesInfo,
	onRefresh,
	onRefreshRepositories,
	onUpgradeAllRequest,
	onReinstallRequest,
	search,
	setSearch,
}: {
	packageRowsData: PackageRowInfo[];
	hiddenUserRepositories: Set<string>;
	repositoriesInfo: TauriRepositoriesInfo | undefined;
	onRefresh: () => void;
	onRefreshRepositories: () => void;
	onUpgradeAllRequest: () => void;
	onReinstallRequest: () => void;
	search: string;
	setSearch: (value: string) => void;
}) {
	const { isLoading } = usePageContext();

	return (
		<div
			className={
				"flex flex-wrap flex-shrink-0 flex-grow-0 flex-row gap-2 items-center"
			}
		>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
				{tc("projects:manage:manage packages")}
			</p>

			<Tooltip>
				<TooltipTrigger>
					<Button
						variant={"ghost"}
						size={"icon"}
						onClick={onRefresh}
						className={"flex-shrink-0"}
						disabled={isLoading}
					>
						{isLoading ? (
							<RefreshCw className="w-5 h-5 animate-spin" />
						) : (
							<RefreshCw className={"w-5 h-5"} />
						)}
					</Button>
				</TooltipTrigger>
				<TooltipContent>
					{tc("projects:manage:tooltip:refresh packages")}
				</TooltipContent>
			</Tooltip>

			<SearchBox
				className={"w-max flex-grow"}
				value={search}
				onChange={(e) => setSearch(e.target.value)}
			/>

			{packageRowsData.some((row) => row.latest.status === "upgradable") && (
				<Button
					className={"flex-shrink-0"}
					onClick={onUpgradeAllRequest}
					disabled={isLoading}
					variant={"success"}
				>
					{tc("projects:manage:button:upgrade all")}
				</Button>
			)}

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant={"ghost"} size={"icon"} className={"flex-shrink-0"}>
						<Ellipsis className={"size-5"} />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent>
					<DropdownMenuItem
						className={"p-3"}
						onClick={onReinstallRequest}
						disabled={isLoading}
					>
						{tc("projects:manage:button:reinstall all")}
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button className={"flex-shrink-0 p-3"}>
						{tc("projects:manage:button:select repositories")}
					</Button>
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
						hideUserLocalPackages={
							repositoriesInfo?.hide_local_user_packages ?? false
						}
						refetch={onRefreshRepositories}
					/>
					<hr className="my-3" />
					{repositoriesInfo?.user_repositories?.map((repository) => (
						<RepositoryMenuItem
							hiddenUserRepositories={hiddenUserRepositories}
							repositoryName={repository.display_name}
							repositoryId={repository.id}
							refetch={onRefreshRepositories}
							key={repository.id}
						/>
					))}
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}

function BulkUpdateCard({
	bulkUpdateMode,
	bulkUpgradeAll,
	bulkRemoveAll,
	bulkInstallAll,
	cancel,
}: {
	bulkUpdateMode: BulkUpdateMode;
	bulkUpgradeAll?: () => void;
	bulkRemoveAll?: () => void;
	bulkInstallAll?: () => void;
	cancel?: () => void;
}) {
	if (bulkUpdateMode === "any") return null;

	const canInstall = bulkUpdateMode === "install";
	const canUpgrade =
		bulkUpdateMode === "upgrade" || bulkUpdateMode === "upgradeOrRemove";
	const canRemove =
		bulkUpdateMode === "remove" || bulkUpdateMode === "upgradeOrRemove";

	return (
		<Card
			className={
				"flex-shrink-0 p-2 flex flex-row gap-2 bg-secondary text-secondary-foreground flex-wrap"
			}
		>
			{canInstall && (
				<ButtonDisabledIfLoading onClick={bulkInstallAll}>
					{tc("projects:manage:button:install selected")}
				</ButtonDisabledIfLoading>
			)}
			{canUpgrade && (
				<ButtonDisabledIfLoading onClick={bulkUpgradeAll} variant={"success"}>
					{tc("projects:manage:button:upgrade selected")}
				</ButtonDisabledIfLoading>
			)}
			{canRemove && (
				<ButtonDisabledIfLoading
					onClick={bulkRemoveAll}
					variant={"destructive"}
				>
					{tc("projects:manage:button:uninstall selected")}
				</ButtonDisabledIfLoading>
			)}
			<ButtonDisabledIfLoading onClick={cancel}>
				{tc("projects:manage:button:clear selection")}
			</ButtonDisabledIfLoading>
		</Card>
	);
}

function RepositoryMenuItem({
	hiddenUserRepositories,
	repositoryName,
	repositoryId,
	refetch,
}: {
	hiddenUserRepositories: Set<string>;
	repositoryName: string;
	repositoryId: string;
	refetch: () => void;
}) {
	const selected = !hiddenUserRepositories.has(repositoryId);
	const onChange = () => {
		if (selected) {
			commands.environmentHideRepository(repositoryId).then(refetch);
		} else {
			commands.environmentShowRepository(repositoryId).then(refetch);
		}
	};

	return (
		<DropdownMenuItem
			className="p-0"
			onSelect={(e) => {
				e.preventDefault();
			}}
		>
			<label
				className={
					"flex cursor-pointer items-center gap-2 p-2 whitespace-normal"
				}
			>
				<Checkbox
					checked={selected}
					onCheckedChange={onChange}
					className="hover:before:content-none"
				/>
				{repositoryName}
			</label>
		</DropdownMenuItem>
	);
}

function UserLocalRepositoryMenuItem({
	hideUserLocalPackages,
	refetch,
}: {
	hideUserLocalPackages: boolean;
	refetch: () => void;
}) {
	const selected = !hideUserLocalPackages;
	const onChange = () => {
		if (selected) {
			commands.environmentSetHideLocalUserPackages(true).then(refetch);
		} else {
			commands.environmentSetHideLocalUserPackages(false).then(refetch);
		}
	};

	return (
		<DropdownMenuItem
			className="p-0"
			onSelect={(e) => {
				e.preventDefault();
			}}
		>
			<label className={"flex cursor-pointer items-center gap-2 p-2"}>
				<Checkbox
					checked={selected}
					onCheckedChange={onChange}
					className="hover:before:content-none"
				/>
				{tc("vpm repositories:source:local")}
			</label>
		</DropdownMenuItem>
	);
}

const PackageRow = memo(function PackageRow({
	pkg,
	onInstallRequested,
	onRemoveRequested,
	bulkUpdateSelected,
	bulkUpdateAvailable,
	addBulkUpdatePackage,
	removeBulkUpdatePackage,
}: {
	pkg: PackageRowInfo;
	onInstallRequested: (
		pkg: TauriPackage,
		hasUnityIncompatibleLatest?: boolean,
	) => void;
	onRemoveRequested: (pkgId: PackageRowInfo) => void;
	bulkUpdateSelected: boolean;
	bulkUpdateAvailable: boolean;
	addBulkUpdatePackage: (pkg: PackageRowInfo) => void;
	removeBulkUpdatePackage: (pkg: PackageRowInfo) => void;
}) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const versionNames = [...pkg.unityCompatible.keys()];
	const latestVersion: string | undefined = versionNames[0];
	useCallback(
		(version: string) => {
			if (
				pkg.installed != null &&
				version === toVersionString(pkg.installed.version)
			)
				return;
			const pkgVersion =
				pkg.unityCompatible.get(version) ?? pkg.unityIncompatible.get(version);
			if (!pkgVersion) return;
			onInstallRequested(pkgVersion);
		},
		[onInstallRequested, pkg],
	);
	const installLatest = () => {
		if (pkg.latest.status === "none") return;
		onInstallRequested(pkg.latest.pkg, pkg.latest.hasUnityIncompatibleLatest);
	};

	const remove = () => {
		onRemoveRequested(pkg);
	};

	const onClickBulkUpdate = () => {
		if (bulkUpdateSelected) {
			removeBulkUpdatePackage(pkg);
		} else {
			addBulkUpdatePackage(pkg);
		}
	};

	return (
		<>
			<td className={`${cellClass} w-1`}>
				<CheckboxDisabledIfLoading
					checked={bulkUpdateSelected}
					onCheckedChange={onClickBulkUpdate}
					disabled={!bulkUpdateAvailable}
					className="hover:before:content-none"
				/>
			</td>
			<td className={`${cellClass} overflow-hidden max-w-80 overflow-ellipsis`}>
				<Tooltip
					open={
						pkg.description ? undefined /* auto */ : false /* disable tooltip */
					}
				>
					<TooltipTrigger asChild>
						<div
							className={`flex flex-col ${pkg.installed ? "" : "opacity-50"}`}
						>
							<p className="font-normal">{pkg.displayName}</p>
							<p className="font-normal opacity-50 text-sm">{pkg.id}</p>
						</div>
					</TooltipTrigger>
					<TooltipContent className={"max-w-[80dvw]"}>
						<p
							className={`whitespace-normal ${pkg.installed ? "" : "opacity-50"}`}
						>
							{pkg.description}
						</p>
					</TooltipContent>
				</Tooltip>
			</td>
			<td className={noGrowCellClass}>
				<PackageVersionSelector
					pkg={pkg}
					onInstallRequested={onInstallRequested}
				/>
			</td>
			<td className={`${cellClass} min-w-32 w-32`}>
				<LatestPackageInfo
					info={pkg.latest}
					onInstallRequested={onInstallRequested}
				/>
			</td>
			<td className={`${noGrowCellClass} max-w-32 overflow-hidden`}>
				{pkg.sources.size === 0 ? (
					pkg.isThereSource ? (
						<p>{tc("projects:manage:source not selected")}</p>
					) : (
						<p>{tc("projects:manage:none")}</p>
					)
				) : pkg.sources.size === 1 ? (
					<Tooltip>
						<TooltipTrigger>
							<p className="overflow-hidden overflow-ellipsis">
								{[...pkg.sources][0]}
							</p>
						</TooltipTrigger>
						<TooltipContent>{[...pkg.sources][0]}</TooltipContent>
					</Tooltip>
				) : (
					<Tooltip>
						<TooltipTrigger>
							<p>{tc("projects:manage:multiple sources")}</p>
						</TooltipTrigger>
						<TooltipContent>{[...pkg.sources].join(", ")}</TooltipContent>
					</Tooltip>
				)}
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					{pkg.installed ? (
						<Tooltip>
							<TooltipTrigger asChild>
								<ButtonDisabledIfLoading
									variant={"ghost"}
									size={"icon"}
									onClick={remove}
								>
									<CircleMinus className={"size-5 text-destructive"} />
								</ButtonDisabledIfLoading>
							</TooltipTrigger>
							<TooltipContent>
								{tc("projects:manage:tooltip:remove packages")}
							</TooltipContent>
						</Tooltip>
					) : (
						<Tooltip>
							<TooltipTrigger asChild>
								<ButtonDisabledIfLoading
									variant={"ghost"}
									size={"icon"}
									disabled={!latestVersion}
									className={
										!latestVersion ? "disabled:pointer-events-auto" : ""
									}
									onClick={installLatest}
								>
									<CirclePlus className={"size-5 text-secondary-foreground"} />
								</ButtonDisabledIfLoading>
							</TooltipTrigger>
							<TooltipContent>
								{!latestVersion
									? tc("projects:manage:tooltip:incompatible with unity")
									: tc("projects:manage:tooltip:add package")}
							</TooltipContent>
						</Tooltip>
					)}
				</div>
			</td>
		</>
	);
});

function bulkUpdateModeForPackage(
	pkg: PackageRowInfo,
): PackageBulkUpdateMode | "nothing" {
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

const PackageVersionSelector = memo(function PackageVersionSelector({
	pkg,
	onInstallRequested,
}: {
	pkg: PackageRowInfo;
	onInstallRequested: (pkg: TauriPackage) => void;
}) {
	const onChange = useCallback(
		(version: string) => {
			if (
				pkg.installed != null &&
				version === toVersionString(pkg.installed.version)
			)
				return;
			const pkgVersion =
				pkg.unityCompatible.get(version) ?? pkg.unityIncompatible.get(version);
			if (!pkgVersion) return;
			onInstallRequested(pkgVersion);
		},
		[
			onInstallRequested,
			pkg.installed,
			pkg.unityCompatible,
			pkg.unityIncompatible,
		],
	);

	const versionNames = [...pkg.unityCompatible.keys()];
	const incompatibleNames = [...pkg.unityIncompatible.keys()];
	const selectedVersion = pkg.installed?.version
		? toVersionString(pkg.installed.version)
		: "";

	const { isLoading } = usePageContext();

	const [isOpen, setIsOpen] = useState(false);

	return (
		<Select
			value={selectedVersion}
			onValueChange={onChange}
			disabled={isLoading}
			open={isOpen}
			onOpenChange={setIsOpen}
		>
			<SelectTrigger>
				<SelectValue
					asChild
					placeholder={<PackageInstalledInfo pkg={pkg} />}
					className={`border-primary/10 ${pkg.installed?.yanked ? "text-destructive" : ""}`}
				>
					<PackageInstalledInfo pkg={pkg} />
				</SelectValue>
			</SelectTrigger>
			<SelectContent>
				{/* PackageVersionList is extremely heavy */}
				{isOpen && (
					<PackageVersionList
						versionNames={versionNames}
						incompatibleNames={incompatibleNames}
					/>
				)}
			</SelectContent>
		</Select>
	);
});

function PackageVersionList({
	versionNames,
	incompatibleNames,
}: {
	versionNames: string[];
	incompatibleNames: string[];
}) {
	return (
		<SelectGroup>
			{versionNames.map((v) => (
				<SelectItem key={v} value={v}>
					{v}
				</SelectItem>
			))}
			{incompatibleNames.length > 0 && versionNames.length > 0 && (
				<SelectSeparator />
			)}
			{incompatibleNames.length > 0 && (
				<SelectLabel>{tc("projects:manage:incompatible packages")}</SelectLabel>
			)}
			{incompatibleNames.map((v) => (
				<SelectItem key={v} value={v}>
					{v}
				</SelectItem>
			))}
		</SelectGroup>
	);
}

function canBulkUpdate(
	bulkUpdateMode: BulkUpdateMode,
	possibleUpdate: PackageBulkUpdateMode | "nothing",
): boolean {
	if (possibleUpdate === "nothing") return false;
	if (bulkUpdateMode === "any") return true;
	if (bulkUpdateMode === possibleUpdate) return true;
	if (bulkUpdateMode === "upgradeOrRemove" && possibleUpdate === "remove")
		return true;
	if (bulkUpdateMode === "upgrade" && possibleUpdate === "upgradeOrRemove")
		return true;
	if (bulkUpdateMode === "remove" && possibleUpdate === "upgradeOrRemove")
		return true;
	return false;
}

function PackageInstalledInfo({
	pkg,
}: {
	pkg: PackageRowInfo;
}) {
	if (pkg.installed) {
		const version = toVersionString(pkg.installed.version);
		if (pkg.installed.yanked) {
			return (
				<p className={"text-destructive"}>
					{version} {tc("projects:manage:yanked")}
				</p>
			);
		} else {
			return <p>{version}</p>;
		}
	} else {
		return (
			<p className="text-muted-foreground/70">{tc("projects:manage:none")}</p>
		);
	}
}

function LatestPackageInfo({
	info,
	onInstallRequested,
}: {
	info: PackageLatestInfo;
	onInstallRequested: (
		pkg: TauriPackage,
		hasUnityIncompatibleLatest?: boolean,
	) => void;
}) {
	switch (info.status) {
		case "none":
			return (
				<p className="text-muted-foreground">{tc("projects:manage:none")}</p>
			);
		case "contains":
			return <p>{toVersionString(info.pkg.version)}</p>;
		case "upgradable":
			return (
				<Tooltip>
					<TooltipTrigger asChild>
						<ButtonDisabledIfLoading
							variant={"outline-success"}
							className={
								"text-left px-2 py-1 w-full h-full font-normal text-base normal-case border-success hover:border-success/70 text-success hover:text-success/70"
							}
							onClick={() =>
								onInstallRequested(info.pkg, info.hasUnityIncompatibleLatest)
							}
						>
							<CircleArrowUp color={"green"} className={"size-4 inline mr-2"} />
							{toVersionString(info.pkg.version)}
						</ButtonDisabledIfLoading>
					</TooltipTrigger>
					<TooltipContent>
						{tc("projects:manage:tooltip:upgrade package")}
					</TooltipContent>
				</Tooltip>
			);
		default:
			assertNever(info);
	}
}
