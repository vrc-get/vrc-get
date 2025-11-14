// noinspection ExceptionCaughtLocallyJS

import {
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import {
	CircleArrowUp,
	CircleMinus,
	CirclePlus,
	Ellipsis,
	RefreshCw,
} from "lucide-react";
import type React from "react";
import {
	memo,
	useCallback,
	useLayoutEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { applyChangesMutation } from "@/app/_main/projects/manage/-use-package-change";
import { Route } from "@/app/_main/projects/manage/index";
import { ExternalLink } from "@/components/ExternalLink";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ScrollArea } from "@/components/ui/scroll-area";
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
import type { TauriPackage, TauriRepositoriesInfo } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import { usePackageUpdateInProgress } from "@/lib/global-events";
import { tc, tt } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { toVersionString } from "@/lib/version";
import type {
	PackageLatestInfo,
	PackageRowInfo,
} from "./-collect-package-row-info";
import {
	ButtonDisabledIfLoading,
	CheckboxDisabledIfLoading,
	DropdownMenuItemDisabledIfLoading,
	usePageContext,
} from "./-page-context";

const environmentRepositoriesInfo = queryOptions({
	queryKey: ["environmentRepositoriesInfo"],
	queryFn: commands.environmentRepositoriesInfo,
});

export const PackageListCard = memo(function PackageListCard({
	packageRowsData,
	repositoriesInfo,
	onRefresh,
}: {
	packageRowsData: PackageRowInfo[];
	repositoriesInfo: TauriRepositoriesInfo | undefined;
	onRefresh: () => void;
}) {
	const [search, setSearch] = useState("");
	const [bulkUpdatePackageIdsRaw, setBulkUpdatePackageIds] = useState<string[]>(
		[],
	);

	const bulkUpdatePackageIds = useMemo(() => {
		const packageIds = new Set(packageRowsData.map((p) => p.id));

		return bulkUpdatePackageIdsRaw.filter((pkgId) => packageIds.has(pkgId));
	}, [packageRowsData, bulkUpdatePackageIdsRaw]);

	useDocumentEvent(
		"post-package-changes",
		() => setBulkUpdatePackageIds([]),
		[],
	);

	const bulkUpdateMode = useMemo(() => {
		const packageRowByPackageId = new Map(
			packageRowsData.map((row) => [row.id, row]),
		);
		return updateModeFromPackageModes(
			bulkUpdatePackageIds.map((id) => {
				const data = packageRowByPackageId.get(id);
				if (data == null) throw new Error(`bad id: ${id}`);
				return bulkUpdateModeForPackage(data);
			}),
		);
	}, [bulkUpdatePackageIds, packageRowsData]);

	const filteredPackageIds = useMemo(() => {
		if (search === "") return new Set<string>(packageRowsData.map((x) => x.id));
		const searchLower = search.toLowerCase();
		return new Set<string>(
			packageRowsData
				.filter(
					(row) =>
						row.displayName.toLowerCase().includes(searchLower) ||
						row.id.toLowerCase().includes(searchLower) ||
						row.keywords.some((alias) =>
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

	const addBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		setBulkUpdatePackageIds((prev) => {
			return prev.some((id) => id === row.id) ? prev : [...prev, row.id];
		});
	}, []);

	const removeBulkUpdatePackage = useCallback((row: PackageRowInfo) => {
		setBulkUpdatePackageIds((prev) => prev.filter((id) => id !== row.id));
	}, []);

	// Fix scroll position when bulk update card visibility is changed
	const scrollTableOuterRef = useRef<HTMLDivElement>(null);
	const scrollTableScrollAreaRef = useRef<HTMLDivElement>(null);
	// TODO: Is this a good way to get BoundingClientRect before dom update?
	const preRenderOuterTop =
		scrollTableOuterRef.current?.getBoundingClientRect()?.top;
	useLayoutEffect(() => {
		if (preRenderOuterTop == null) return;
		if (scrollTableOuterRef.current == null) return;
		if (scrollTableScrollAreaRef.current == null) return;
		const postRenderOuterTop =
			scrollTableOuterRef.current.getBoundingClientRect()?.top;
		const heightDiff = postRenderOuterTop - preRenderOuterTop;
		if (heightDiff === 0) return;
		scrollTableScrollAreaRef.current.scrollBy({
			top: heightDiff,
			behavior: "instant",
		});
	});

	const dialogForState: React.ReactNode = null;

	const TABLE_HEAD = [
		"projects:manage:package",
		"projects:manage:installed",
		"projects:manage:latest",
		"general:source",
	];

	return (
		<Card className="grow shrink flex shadow-none w-full">
			<CardContent className="w-full p-2 flex flex-col gap-2 compact:p-1 compact:gap-1.5">
				<ManagePackagesHeading
					packageRowsData={packageRowsData}
					hiddenUserRepositories={hiddenUserRepositories}
					repositoriesInfo={repositoriesInfo}
					onRefresh={onRefresh}
					search={search}
					setSearch={setSearch}
				/>
				<BulkUpdateCard
					bulkUpdateMode={bulkUpdateMode}
					bulkUpdatePackageIds={bulkUpdatePackageIds}
					packageRowsData={packageRowsData}
					cancel={() => setBulkUpdatePackageIds([])}
				/>
				<ScrollableCardTable
					className={"h-full rounded-md"}
					ref={scrollTableOuterRef}
					viewportRef={scrollTableScrollAreaRef}
				>
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
										"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground px-2.5 py-1.5"
									}
								>
									<small className="font-normal leading-none">{tc(head)}</small>
								</th>
							))}
							<th
								className={
									"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground px-2.5 py-1.5"
								}
							/>
						</tr>
					</thead>
					<tbody>
						{packageRowsData.map((row) => (
							<tr
								className="even:bg-secondary/30 anchor-none"
								hidden={!filteredPackageIds.has(row.id)}
								key={row.id}
							>
								<PackageRow
									pkg={row}
									bulkUpdateSelected={bulkUpdatePackageIds.some(
										(id) => id === row.id,
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
	search,
	setSearch,
}: {
	packageRowsData: PackageRowInfo[];
	hiddenUserRepositories: Set<string>;
	repositoriesInfo: TauriRepositoriesInfo | undefined;
	onRefresh: () => void;
	search: string;
	setSearch: (value: string) => void;
}) {
	const { isLoading } = usePageContext();
	const backgroundLoading = usePackageUpdateInProgress();

	const queryClient = useQueryClient();

	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

	const setShowPrereleasePackages = useMutation({
		mutationFn: async (shown: boolean) => {
			await commands.environmentSetShowPrereleasePackages(shown);
		},
		onMutate: async (shown) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					hide_local_user_packages: shown,
				});
			}
			return data;
		},
		onError: (e, _, ctx) => {
			reportError(e);
			console.error(e);
			queryClient.setQueryData(environmentRepositoriesInfo.queryKey, ctx);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentRepositoriesInfo);
		},
	});

	const onUpgradeAllRequest = (stable: boolean) => {
		const latestKey = stable ? "stableLatest" : "latest";
		const packagesToInstall = packageRowsData
			.map((row) => row[latestKey])
			.filter<PackageLatestInfo & { status: "upgradable" }>(
				(latest) => latest.status === "upgradable",
			);
		const packages: TauriPackage[] = packagesToInstall.map(
			(latest) => latest.pkg,
		);
		const hasUnityIncompatibleLatest = packagesToInstall.some(
			(latest) => latest.hasUnityIncompatibleLatest,
		);
		packageChange.mutate({
			type: "upgradeAll",
			hasUnityIncompatibleLatest,
			packages,
		});
	};
	const upgradableToLatest = packageRowsData.some(
		(row) => row.latest.status === "upgradable",
	);
	const upgradingToPrerelease = packageRowsData.some(
		(row) =>
			row.latest.status === "upgradable" && row.latest.pkg.version.pre !== "",
	);
	const upgradableToStable =
		packageRowsData.some((row) => row.stableLatest.status === "upgradable") &&
		upgradingToPrerelease;

	const searchRef = useRef<HTMLInputElement>(null);

	useDocumentEvent(
		"keydown",
		(e) => {
			if (isFindKey(e)) {
				searchRef.current?.focus();
			}
		},
		[],
	);

	return (
		<div
			className={
				"flex flex-wrap shrink-0 grow-0 flex-row gap-2 items-center gap-y-1"
			}
		>
			<p className="cursor-pointer font-bold grow-0 shrink-0 pl-2">
				{tc("projects:manage:manage packages")}
			</p>

			<Tooltip>
				<TooltipTrigger>
					<Button
						variant={"ghost"}
						size={"icon"}
						onClick={onRefresh}
						className="shrink-0"
						disabled={isLoading || backgroundLoading}
					>
						{isLoading || backgroundLoading ? (
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
				className={"w-max grow"}
				value={search}
				onChange={(e) => setSearch(e.target.value)}
				ref={searchRef}
			/>

			{upgradableToLatest && (
				<Button
					className="shrink-0"
					onClick={() => onUpgradeAllRequest(false)}
					disabled={isLoading}
					variant={"success"}
				>
					{tc("projects:manage:button:upgrade all")}
				</Button>
			)}

			{/* show this button only if some packages are upgradable to prerelease and there is different stable */}
			{upgradableToStable && (
				<Button
					className="shrink-0"
					onClick={() => onUpgradeAllRequest(true)}
					disabled={isLoading}
					variant={"success"}
				>
					{tc("projects:manage:button:upgrade all stable")}
				</Button>
			)}

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant={"ghost"} size={"icon"} className={"shrink-0"}>
						<Ellipsis className={"size-5"} />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent>
					<DropdownMenuItem
						className={"p-3"}
						onClick={() => packageChange.mutate({ type: "reinstallAll" })}
						disabled={isLoading}
					>
						{tc("projects:manage:button:reinstall all")}
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button className={"shrink-0 p-3"}>
						{tc("projects:manage:button:package filters")}
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent
					className={"max-h-96 w-64 overflow-y-hidden flex flex-col"}
				>
					<DropdownMenuLabel>
						{tc("projects:manage:menu:repository filter")}
					</DropdownMenuLabel>
					<ScrollArea className={"flex flex-col"}>
						<RepositoryMenuItem
							hiddenUserRepositories={hiddenUserRepositories}
							repositoryName={tt("vpm repositories:source:official")}
							repositoryId={"com.vrchat.repos.official"}
						/>
						<RepositoryMenuItem
							hiddenUserRepositories={hiddenUserRepositories}
							repositoryName={tt("vpm repositories:source:curated")}
							repositoryId={"com.vrchat.repos.curated"}
						/>
						<UserLocalRepositoryMenuItem
							hideUserLocalPackages={
								repositoriesInfo?.hide_local_user_packages ?? false
							}
						/>
						<hr className="my-1.5" />
						{repositoriesInfo?.user_repositories?.map((repository) => (
							<RepositoryMenuItem
								hiddenUserRepositories={hiddenUserRepositories}
								repositoryName={repository.display_name}
								repositoryId={repository.id}
								key={repository.id}
							/>
						))}
					</ScrollArea>
					<DropdownMenuLabel>
						{tc("projects:manage:menu:other filters")}
					</DropdownMenuLabel>
					<DropdownMenuCheckboxItem
						checked={repositoriesInfo?.show_prerelease_packages}
						onClick={(e) => {
							e.preventDefault();
							setShowPrereleasePackages.mutate(
								!repositoriesInfo?.show_prerelease_packages,
							);
						}}
					>
						{tc("settings:show prerelease")}
					</DropdownMenuCheckboxItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}

// if installed and not latest, can upgrade, reinstall or remove
// if installed, can reinstall or remove
// if not installed, can install
// we combined install and upgrade so we can:
// - install or upgrade if not installed or not latest
// - remove or reinstall if installed

const possibleUpdateKind = [
	"canInstallOrUpgrade",
	"canReinstallOrRemove",
	"canInstallOrUpgradeStable",
] as const;

type BulkUpdateModeBase = {
	[k in (typeof possibleUpdateKind)[number]]: boolean;
};
type PackageBulkUpdateMode = BulkUpdateModeBase & { latestIsStable: boolean };
type BulkUpdateMode = BulkUpdateModeBase & { hasPackages: boolean };

function updateModeFromPackageModes(
	map: PackageBulkUpdateMode[],
): BulkUpdateMode {
	let canInstallOrUpgrade = true;
	let canReinstallOrRemove = true;
	let canInstallOrUpgradeStable = true;
	let allLatestIsStable = true;

	for (const mode of map) {
		canInstallOrUpgrade &&= mode.canInstallOrUpgrade;
		canReinstallOrRemove &&= mode.canReinstallOrRemove;
		canInstallOrUpgradeStable &&= mode.canInstallOrUpgradeStable;
		allLatestIsStable &&= mode.latestIsStable;
	}

	return {
		canInstallOrUpgrade,
		canReinstallOrRemove,
		// if all packages latest is stable, installOrUpgrade is same as installOrUpgradeStable
		canInstallOrUpgradeStable: !allLatestIsStable && canInstallOrUpgradeStable,
		hasPackages: map.length > 0,
	};
}

function bulkUpdateModeForPackage(pkg: PackageRowInfo): PackageBulkUpdateMode {
	const canReinstallOrRemove: boolean = pkg.installed != null;
	// there is possibility for installed and latest is newest or newer than latest
	const canInstallOrUpgrade: boolean = pkg.installed
		? pkg.latest.status === "upgradable"
		: pkg.latest.status !== "none";
	const canInstallOrUpgradeStable: boolean = pkg.installed
		? pkg.stableLatest.status === "upgradable"
		: pkg.stableLatest.status !== "none";
	const latestIsStable =
		pkg.latest.status !== "none" && pkg.latest.pkg.version.pre === "";
	return {
		canInstallOrUpgrade,
		canReinstallOrRemove,
		canInstallOrUpgradeStable,
		latestIsStable,
	};
}

function canBulkUpdate(
	bulkUpdateMode: BulkUpdateMode,
	possibleUpdate: PackageBulkUpdateMode,
): boolean {
	// if either is available, we can bulk update
	for (const kind of possibleUpdateKind) {
		if (bulkUpdateMode[kind] && possibleUpdate[kind]) return true;
	}
	return false;
}

function BulkUpdateCard({
	bulkUpdateMode,
	bulkUpdatePackageIds,
	packageRowsData,
	cancel,
}: {
	bulkUpdateMode: BulkUpdateMode;
	bulkUpdatePackageIds: string[];
	packageRowsData: PackageRowInfo[];
	cancel?: () => void;
}) {
	const count = bulkUpdatePackageIds.length;
	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

	const bulkRemoveAll = () => {
		packageChange.mutate({
			type: "bulkRemoved",
			packageIds: bulkUpdatePackageIds,
		});
	};

	const bulkReinstallAll = () => {
		packageChange.mutate({
			type: "bulkReinstalled",
			packageIds: bulkUpdatePackageIds,
		});
	};

	const bulkInstallOrUpgradeAll = (stable: boolean) => {
		const latestKey = stable ? "stableLatest" : "latest";

		const packageRowByPackageId = new Map(
			packageRowsData.map((r) => [r.id, r]),
		);

		let packagesToInstall: (PackageLatestInfo & {
			status: "contains" | "upgradable";
		})[];

		try {
			packagesToInstall = bulkUpdatePackageIds.map((id) => {
				const data = packageRowByPackageId.get(id);
				if (data == null) throw new Error(`bad id: ${id}`);
				const latestInfo = data[latestKey];
				if (latestInfo.status === "none")
					throw new Error("Package is not installable");
				return latestInfo;
			});
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			return;
		}

		const packages = packagesToInstall.map((info) => info.pkg);
		const hasUnityIncompatibleLatest = packagesToInstall.some(
			(info) => info.hasUnityIncompatibleLatest,
		);

		packageChange.mutate({
			type: "bulkInstalled",
			hasUnityIncompatibleLatest,
			packages,
		});
	};

	if (!bulkUpdateMode.hasPackages) return null;
	return (
		<Card
			className={
				"shrink-0 p-2 flex flex-row gap-2 bg-secondary text-secondary-foreground flex-wrap"
			}
		>
			{bulkUpdateMode.canInstallOrUpgrade && (
				<ButtonDisabledIfLoading
					onClick={() => bulkInstallOrUpgradeAll?.(false)}
				>
					{tc("projects:manage:button:install selected latest")}
				</ButtonDisabledIfLoading>
			)}
			{bulkUpdateMode.canInstallOrUpgradeStable && (
				<ButtonDisabledIfLoading
					onClick={() => bulkInstallOrUpgradeAll?.(true)}
				>
					{tc("projects:manage:button:install selected stable latest")}
				</ButtonDisabledIfLoading>
			)}
			{bulkUpdateMode.canReinstallOrRemove && (
				<ButtonDisabledIfLoading onClick={bulkReinstallAll}>
					{tc("projects:manage:button:reinstall selected")}
				</ButtonDisabledIfLoading>
			)}
			{bulkUpdateMode.canReinstallOrRemove && (
				<ButtonDisabledIfLoading
					onClick={bulkRemoveAll}
					variant={"destructive"}
				>
					{tc("projects:manage:button:uninstall selected")}
				</ButtonDisabledIfLoading>
			)}
			<ButtonDisabledIfLoading onClick={cancel}>
				{tc("projects:manage:button:clear selection")}
				{" ("}
				{tc("projects:manage:n packages selected", { count })}
				{")"}
			</ButtonDisabledIfLoading>
		</Card>
	);
}

const preventDefault = (e: Event) => e.preventDefault();

function RepositoryMenuItem({
	hiddenUserRepositories,
	repositoryName,
	repositoryId,
}: {
	hiddenUserRepositories: Set<string>;
	repositoryName: string;
	repositoryId: string;
}) {
	const selected = !hiddenUserRepositories.has(repositoryId);

	const queryClient = useQueryClient();

	const setHideRepository = useMutation({
		mutationFn: async ({ id, shown }: { id: string; shown: boolean }) => {
			if (shown) {
				await commands.environmentShowRepository(id);
			} else {
				await commands.environmentHideRepository(id);
			}
		},
		onMutate: async ({ id, shown }: { id: string; shown: boolean }) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				let hidden_user_repositories: string[];
				if (shown) {
					if (data.hidden_user_repositories.includes(id)) {
						hidden_user_repositories = data.hidden_user_repositories;
					} else {
						hidden_user_repositories = [...data.hidden_user_repositories, id];
					}
				} else {
					hidden_user_repositories = data.hidden_user_repositories.filter(
						(x) => x !== id,
					);
				}

				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					hidden_user_repositories,
				});
			}
			return data;
		},
		onError: (e, _, ctx) => {
			reportError(e);
			console.error(e);
			queryClient.setQueryData(environmentRepositoriesInfo.queryKey, ctx);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentRepositoriesInfo);
		},
	});

	return (
		<DropdownMenuCheckboxItem
			checked={selected}
			onCheckedChange={(shown) =>
				setHideRepository.mutate({ id: repositoryId, shown })
			}
			onSelect={preventDefault}
		>
			{repositoryName}
		</DropdownMenuCheckboxItem>
	);
}

function UserLocalRepositoryMenuItem({
	hideUserLocalPackages,
}: {
	hideUserLocalPackages: boolean;
}) {
	const selected = !hideUserLocalPackages;

	const queryClient = useQueryClient();

	const setHideLocalUserPackages = useMutation({
		mutationFn: async (shown: boolean) => {
			await commands.environmentSetHideLocalUserPackages(shown);
		},
		onMutate: async (shown) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					hide_local_user_packages: shown,
				});
			}
			return data;
		},
		onError: (e, _, ctx) => {
			reportError(e);
			console.error(e);
			queryClient.setQueryData(environmentRepositoriesInfo.queryKey, ctx);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentRepositoriesInfo);
		},
	});

	return (
		<DropdownMenuCheckboxItem
			checked={selected}
			onCheckedChange={(x) => setHideLocalUserPackages.mutate(x)}
			onSelect={preventDefault}
		>
			{tc("vpm repositories:source:local")}
		</DropdownMenuCheckboxItem>
	);
}

const PackageRow = memo(function PackageRow({
	pkg,
	bulkUpdateSelected,
	bulkUpdateAvailable,
	addBulkUpdatePackage,
	removeBulkUpdatePackage,
}: {
	pkg: PackageRowInfo;
	bulkUpdateSelected: boolean;
	bulkUpdateAvailable: boolean;
	addBulkUpdatePackage: (pkg: PackageRowInfo) => void;
	removeBulkUpdatePackage: (pkg: PackageRowInfo) => void;
}) {
	const cellClass = "p-3.5 compact:py-1";
	const noGrowCellClass = `${cellClass} w-1`;
	const versionNames = [...pkg.unityCompatible.keys()];
	const latestVersion: string | undefined = versionNames[0];

	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

	const installLatest = () => {
		if (pkg.latest.status === "none") return;

		packageChange.mutate({
			type: "install",
			pkg: pkg.latest.pkg,
			hasUnityIncompatibleLatest: pkg.latest.hasUnityIncompatibleLatest,
		});
	};

	const reinstallPackage = () => {
		if (pkg.installed == null) return;
		packageChange.mutate({
			type: "bulkReinstalled",
			packageIds: [pkg.id],
		});
	};

	const remove = () => {
		packageChange.mutate({
			type: "remove",
			displayName: pkg.displayName,
			packageId: pkg.id,
		});
	};

	const onClickBulkUpdate = () => {
		if (bulkUpdateSelected) {
			removeBulkUpdatePackage(pkg);
		} else {
			addBulkUpdatePackage(pkg);
		}
	};

	const documentationUrl = pkg.documentationUrl
		? pkg.documentationUrl.url
		: null;
	const changelogUrl = pkg.changelogUrl ? pkg.changelogUrl.url : null;

	return (
		<>
			<td className={`${cellClass} w-1 compact:px-2`}>
				<div className={"flex content-center aspect-square"}>
					<CheckboxDisabledIfLoading
						checked={bulkUpdateSelected}
						onCheckedChange={onClickBulkUpdate}
						disabled={!bulkUpdateAvailable}
						className="hover:before:content-none"
					/>
				</div>
			</td>
			<td className={`${cellClass} overflow-hidden max-w-80 text-ellipsis`}>
				<Tooltip>
					<TooltipTrigger asChild>
						<div
							className={`flex flex-col ${pkg.installed ? "" : "opacity-50"}`}
						>
							<p className="font-normal">{pkg.displayName}</p>
							<p className="font-normal opacity-50 text-sm compact:hidden">
								{pkg.id}
							</p>
						</div>
					</TooltipTrigger>
					<TooltipContent className={"max-w-[80dvw]"}>
						<p
							className={`whitespace-normal ${pkg.installed ? "" : "opacity-50"}`}
						>
							{pkg.description}
						</p>
						<p className="font-normal opacity-50 text-sm">{pkg.id}</p>
					</TooltipContent>
				</Tooltip>
			</td>
			<td className={noGrowCellClass}>
				<PackageVersionSelector pkg={pkg} />
			</td>
			<td className={`${cellClass} min-w-32 w-32`}>
				<LatestPackageInfo info={pkg.latest} />
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
							<p className="overflow-hidden text-ellipsis">
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
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant={"ghost"} size={"icon"}>
								<Ellipsis className={"size-5"} />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent>
							<DropdownMenuItemDisabledIfLoading
								onClick={reinstallPackage}
								disabled={pkg.installed == null}
							>
								{tc("projects:manage:menuitem:reinstall")}
							</DropdownMenuItemDisabledIfLoading>
							<DropdownMenuItem disabled={changelogUrl == null}>
								<ExternalLink href={changelogUrl ?? ""}>
									{tc("projects:manage:menuitem:see changelog")}
								</ExternalLink>
							</DropdownMenuItem>
							<DropdownMenuItem disabled={documentationUrl == null}>
								<ExternalLink href={documentationUrl ?? ""}>
									{tc("projects:manage:menuitem:see documentation")}
								</ExternalLink>
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</td>
		</>
	);
});

const PackageVersionSelector = memo(function PackageVersionSelector({
	pkg,
}: {
	pkg: PackageRowInfo;
}) {
	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

	const onChange = (version: string) => {
		if (
			pkg.installed != null &&
			version === toVersionString(pkg.installed.version)
		)
			return;
		const pkgVersion =
			pkg.unityCompatible.get(version) ?? pkg.unityIncompatible.get(version);
		if (!pkgVersion) return;
		packageChange.mutate({
			type: "install",
			pkg: pkgVersion,
		});
	};

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
			<SelectContent className="max-h-[min(24rem,45vh)]">
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

function PackageInstalledInfo({ pkg }: { pkg: PackageRowInfo }) {
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

function LatestPackageInfo({ info }: { info: PackageLatestInfo }) {
	const { projectPath } = Route.useSearch();
	const packageChange = useMutation(applyChangesMutation(projectPath));

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
								packageChange.mutate({
									type: "install",
									pkg: info.pkg,
									hasUnityIncompatibleLatest: info.hasUnityIncompatibleLatest,
								})
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
