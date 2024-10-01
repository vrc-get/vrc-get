"use client";

import { HNavBar, VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
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
import { useBackupProjectModal } from "@/lib/backup-project";
import type { TauriProjectDetails, TauriUnityVersions } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { VRCSDK_PACKAGES, VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { tc } from "@/lib/i18n";
import { nameFromPath } from "@/lib/os";
import { useRemoveProjectModal } from "@/lib/remove-project";
import { toastSuccess, toastThrownError } from "@/lib/toast";
import { useOpenUnity } from "@/lib/use-open-unity";
import { compareUnityVersionString } from "@/lib/version";
import {
	type UseQueryResult,
	useQueries,
	useQuery,
} from "@tanstack/react-query";
import { ArrowLeft, ChevronDown } from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import type React from "react";
import { Suspense, useCallback, useMemo, useState } from "react";
import { combinePackagesAndProjectDetails } from "./collect-package-row-info";
import { PackageListCard } from "./package-list-card";
import { PageContextProvider } from "./page-context";
import {
	useUnity2022Migration,
	useUnity2022PatchMigration,
	useUnityVersionChange,
} from "./unity-migration";
import { usePackageChangeDialog } from "./use-package-change";

export default function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

function PageBody() {
	const searchParams = useSearchParams();
	const router = useRouter();

	const projectRemoveModal = useRemoveProjectModal({
		onRemoved: () => router.back(),
	});
	const backupProjectModal = useBackupProjectModal();

	const projectPath = searchParams.get("projectPath") ?? "";
	const projectName = nameFromPath(projectPath);

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

	const [manualRefetching, setManualRefething] = useState<boolean>(false);

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

	const onRefresh = useCallback(async () => {
		try {
			setManualRefething(true);
			await commands.environmentRefetchPackages();
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
		packagesResult.refetch(); // package changes require package list to be refreshed
	}, [detailsResult, packagesResult]);

	const packageChangeDialog = usePackageChangeDialog({
		projectPath,
		onRefreshProject,
		packageRowsData,
		existingPackages: detailsResult.data?.installed_packages,
	});

	const unity2022Migration = useUnity2022Migration({
		projectPath,
		refresh: onRefresh,
	});

	const unity2022PatchMigration = useUnity2022PatchMigration({
		projectPath,
		refresh: onRefresh,
	});

	const onRefreshRepositories = useCallback(() => {
		repositoriesInfo.refetch();
	}, [repositoriesInfo]);

	const onRemoveProject = useCallback(() => {
		projectRemoveModal.startRemove({
			path: projectPath,
			name: projectName,
			is_exists: true,
		});
	}, [projectName, projectPath, projectRemoveModal]);

	const onBackupProject = useCallback(() => {
		backupProjectModal.startBackup({
			path: projectPath,
			name: projectName,
		});
	}, [backupProjectModal, projectName, projectPath]);

	const onResolveRequest = useCallback(() => {
		packageChangeDialog.createChanges(
			{ type: "resolve" },
			commands.projectResolve(projectPath),
		);
	}, [packageChangeDialog, projectPath]);

	const isLoading =
		packagesResult.isFetching ||
		detailsResult.isFetching ||
		repositoriesInfo.isFetching ||
		unityVersionsResult.isLoading ||
		packageChangeDialog.installingPackage ||
		manualRefetching;

	console.log(`rerender: isloading: ${isLoading}`);

	function checkIfMigrationTo2022Recommended(data: TauriProjectDetails) {
		if (data.unity == null) return false;
		// migrate if the project is using 2019 and has vrcsdk
		if (data.unity[0] !== 2019) return false;
		return data.installed_packages.some(([id, _]) =>
			VRCSDK_PACKAGES.includes(id),
		);
	}

	function checkIf2022PatchMigrationRecommended(
		data: TauriProjectDetails,
		unityData: TauriUnityVersions,
	) {
		if (
			!data.installed_packages.some(([id, _]) => VRCSDK_PACKAGES.includes(id))
		)
			return false;

		if (data.unity == null) return false;
		if (data.unity[0] !== 2022) return false;
		// unity patch is 2022.
		return data.unity_str !== unityData.recommended_version;
	}

	const isResolveRecommended = detailsResult?.data?.should_resolve;
	const isMigrationTo2022Recommended =
		detailsResult.status === "success" &&
		checkIfMigrationTo2022Recommended(detailsResult.data);
	const is2022PatchMigrationRecommended =
		detailsResult.status === "success" &&
		unityVersionsResult.status === "success" &&
		checkIf2022PatchMigrationRecommended(
			detailsResult.data,
			unityVersionsResult.data,
		);

	const pageContext = useMemo(() => ({ isLoading }), [isLoading]);

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
				<Card
					className={"flex-shrink-0 p-2 flex flex-row flex-wrap items-center"}
				>
					<p className="cursor-pointer py-1.5 font-bold flex-grow flex-shrink overflow-hidden basis-52">
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
					<div className={"flex-grow-0 flex-shrink-0 w-2"} />
					<div className="flex-grow-0 flex-shrink-0 flex flex-row items-center">
						<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
							{tc("projects:manage:unity version")}
						</p>
						<div className={"flex-grow-0 flex-shrink-0"}>
							<UnityVersionSelector
								disabled={isLoading}
								projectPath={projectPath}
								detailsResult={detailsResult}
								unityVersions={unityVersionsResult.data}
							/>
						</div>
					</div>
				</Card>
				{isResolveRecommended && (
					<SuggestResolveProjectCard
						disabled={isLoading}
						onResolveRequested={onResolveRequest}
					/>
				)}
				{isMigrationTo2022Recommended && (
					<SuggestMigrateTo2022Card
						disabled={isLoading}
						onMigrateRequested={() => unity2022Migration.request({})}
					/>
				)}
				{is2022PatchMigrationRecommended && (
					<Suggest2022PatchMigrationCard
						disabled={isLoading}
						onMigrateRequested={() => unity2022PatchMigration.request({})}
					/>
				)}
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

function UnityVersionSelector({
	disabled,
	projectPath,
	detailsResult,
	unityVersions,
}: {
	disabled?: boolean;
	projectPath: string;
	detailsResult: UseQueryResult<TauriProjectDetails>;
	unityVersions?: TauriUnityVersions;
}) {
	const unityChangeVersion = useUnityVersionChange({
		projectPath,
		refresh: () => detailsResult.refetch(),
	});

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

	const onChange = useCallback(
		async (version: string) => {
			const detailsData = detailsResult.data;
			if (detailsData == null) return;
			const currentUnityVersion = detailsData.unity_str;
			if (currentUnityVersion == null) return;
			unityChangeVersion.request({
				version,
				isVRCProject,
				currentUnityVersion,
			});
		},
		[detailsResult.data, isVRCProject, unityChangeVersion],
	);

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
			onValueChange={onChange}
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
			{unityChangeVersion.dialog}
		</Select>
	);
}

function SuggestResolveProjectCard({
	disabled,
	onResolveRequested,
}: {
	disabled?: boolean;
	onResolveRequested: () => void;
}) {
	return (
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest resolve")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"} />
			<Button
				variant={"ghost-destructive"}
				onClick={onResolveRequested}
				disabled={disabled}
			>
				{tc("projects:manage:button:resolve")}
			</Button>
		</Card>
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
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest unity migration")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"} />
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
		<Card className={"flex-shrink-0 p-2 flex flex-row items-center"}>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink overflow-hidden whitespace-normal text-sm">
				{tc("projects:manage:suggest unity patch migration")}
			</p>
			<div className={"flex-grow flex-shrink-0 w-2"} />
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
	projectName,
	projectPath,
	unityVersion,
	unityRevision,
	onRemove,
	onBackup,
}: {
	className?: string;
	projectName: string;
	projectPath: string;
	unityVersion: string | null;
	unityRevision: string | null;
	onRemove?: () => void;
	onBackup?: () => void;
}) {
	const openUnity = useOpenUnity();
	const [openLaunchOptions, setOpenLaunchOptions] = useState<
		| false
		| {
				initialArgs: null | string[];
				defaultArgs: string[];
		  }
	>(false);

	const onChangeLaunchOptions = async () => {
		const initialArgs = await commands.projectGetCustomUnityArgs(projectPath);
		const defaultArgs = await commands.environmentGetDefaultUnityArguments();
		setOpenLaunchOptions({
			initialArgs,
			defaultArgs,
		});
	};
	const closeChangeLaunchOptions = () => {
		setOpenLaunchOptions(false);
	};

	return (
		<HNavBar className={className}>
			<Tooltip>
				<TooltipTrigger asChild>
					<Button
						variant={"ghost"}
						size={"icon"}
						onClick={() => history.back()}
					>
						<ArrowLeft className={"w-5 h-5"} />
					</Button>
				</TooltipTrigger>
				<TooltipContent>
					{tc("projects:manage:tooltip:back to projects")}
				</TooltipContent>
			</Tooltip>

			<p className="cursor-pointer py-1.5 font-bold flex-grow-0 whitespace-pre">
				{projectName}
			</p>

			<div className="relative flex gap-2 w-max flex-grow" />

			<DropdownMenu>
				<div className={"flex divide-x"}>
					<Button
						onClick={() =>
							openUnity.openUnity(projectPath, unityVersion, unityRevision)
						}
						className={"rounded-r-none pl-4 pr-3"}
					>
						{tc("projects:button:open unity")}
					</Button>
					<DropdownMenuTrigger asChild className={"rounded-l-none pl-2 pr-2"}>
						<Button>
							<ChevronDown className={"w-4 h-4"} />
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
			{openLaunchOptions !== false && (
				<DialogOpen>
					<LaunchSettings
						projectPath={projectPath}
						initialValue={openLaunchOptions.initialArgs}
						defaultUnityArgs={openLaunchOptions.defaultArgs}
						close={closeChangeLaunchOptions}
					/>
				</DialogOpen>
			)}
		</HNavBar>
	);
}

function LaunchSettings({
	projectPath,
	defaultUnityArgs,
	initialValue,
	close,
}: {
	projectPath: string;
	defaultUnityArgs: string[];
	initialValue: string[] | null;
	close: () => void;
}) {
	const context = useUnityArgumentsSettings(initialValue, defaultUnityArgs);

	const saveAndClose = async () => {
		await commands.projectSetCustomUnityArgs(projectPath, context.currentValue);
		close();
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

function DropdownMenuContentBody({
	projectPath,
	onRemove,
	onBackup,
	onChangeLaunchOptions,
}: {
	projectPath: string;
	onRemove?: () => void;
	onBackup?: () => void;
	onChangeLaunchOptions?: () => void;
}) {
	const openProjectFolder = () =>
		commands.utilOpen(projectPath, "ErrorIfNotExists");
	const forgetUnity = async () => {
		try {
			await commands.projectSetUnityPath(projectPath, null);
			toastSuccess(tc("projects:toast:forgot unity path"));
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};
	const unityPathQuery = useQuery({
		queryFn: () => commands.projectGetUnityPath(projectPath),
		queryKey: ["projectGetUnityPath", projectPath],
		refetchOnWindowFocus: false,
	});

	const unityPath = unityPathQuery.data;

	return (
		<>
			<DropdownMenuItem onClick={onChangeLaunchOptions}>
				{tc("projects:menuitem:change launch options")}
			</DropdownMenuItem>
			{unityPath && (
				<DropdownMenuItem onClick={forgetUnity}>
					{tc("projects:menuitem:forget unity path")}
				</DropdownMenuItem>
			)}
			<DropdownMenuItem onClick={openProjectFolder}>
				{tc("projects:menuitem:open directory")}
			</DropdownMenuItem>
			<DropdownMenuItem onClick={onBackup}>
				{tc("projects:menuitem:backup")}
			</DropdownMenuItem>
			<DropdownMenuItem
				onClick={onRemove}
				className={"bg-destructive text-destructive-foreground"}
			>
				{tc("projects:remove project")}
			</DropdownMenuItem>
		</>
	);
}
