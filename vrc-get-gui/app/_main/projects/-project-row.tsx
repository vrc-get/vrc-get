import {
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
	CircleHelp,
	CircleUserRound,
	Ellipsis,
	Globe,
	Star,
} from "lucide-react";
import React, { type ComponentProps, useContext } from "react";
import { copyProject } from "@/app/_main/projects/manage/-copy-project";
import { MigrationCopyingDialog } from "@/app/_main/projects/manage/-unity-migration";
import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { OpenUnityButton } from "@/components/OpenUnityButton";
import { RemoveProjectDialog } from "@/components/RemoveProjectDialog";
import { Button } from "@/components/ui/button";
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
	Tooltip,
	TooltipContent,
	TooltipPortal,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { assertNever } from "@/lib/assert-never";
import type { TauriProject, TauriProjectType } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { dateToString, formatDateOffset } from "@/lib/dateToString";
import { type DialogContext, openSingleDialog, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { router } from "@/lib/main";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { compareUnityVersionString } from "@/lib/version";

export const ProjectDisplayType: Record<
	TauriProjectType,
	"avatars" | "worlds" | "sdk2" | "unknown"
> = {
	Unknown: "unknown",
	LegacySdk2: "sdk2",
	LegacyWorlds: "worlds",
	LegacyAvatars: "avatars",
	UpmWorlds: "worlds",
	UpmAvatars: "avatars",
	UpmStarter: "unknown",
	Worlds: "worlds",
	Avatars: "avatars",
	VpmStarter: "unknown",
};

export const LegacyProjectTypes = [
	"LegacySdk2",
	"LegacyWorlds",
	"LegacyAvatars",
	"UpmWorlds",
	"UpmAvatars",
	"UpmStarter",
];

const environmentProjects = queryOptions({
	queryKey: ["environmentProjects"],
	queryFn: commands.environmentProjects,
});

export function ProjectRow({
	project,
	loading,
}: {
	project: TauriProject;
	loading?: boolean;
}) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const typeIconClass = "w-5 h-5";

	const { projectTypeKind, displayType, isLegacy, lastModified } =
		getProjectDisplayInfo(project);

	const openProjectFolder = () =>
		commands.utilOpen(project.path, "ErrorIfNotExists");

	const onCopyProject = async () => {
		try {
			await copyProject(project.path);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const setProjectFavorite = useSetProjectFavoriteMutation();

	const removed = !project.is_exists;
	const is_valid = project.is_valid;

	return (
		<ProjectContext.Provider
			value={{ removed, is_valid, loading: Boolean(loading) }}
		>
			<tr
				className={`group even:bg-secondary/30 ${removed || loading || !(project.is_valid ?? true) ? "opacity-50" : ""}`}
			>
				<td className={`${cellClass} w-3`}>
					<div className={"relative flex"}>
						<FavoriteToggleButton
							project={project}
							disabled={removed || loading}
							onToggle={() =>
								setProjectFavorite.mutate({
									...project,
									favorite: !project.favorite,
								})
							}
						/>
					</div>
				</td>
				<td className={`${cellClass} max-w-64 overflow-hidden`}>
					<Tooltip>
						<TooltipTriggerIfInvalid
							className={"text-left select-text cursor-auto w-full"}
						>
							<div className="flex flex-col">
								<Tooltip>
									<TooltipTriggerIfValid
										className={"text-left select-text cursor-auto w-full"}
									>
										<p className="font-normal whitespace-pre">{project.name}</p>
									</TooltipTriggerIfValid>
									<TooltipContent>{project.name}</TooltipContent>
								</Tooltip>
								<Tooltip>
									<TooltipTriggerIfValid
										className={"text-left select-text cursor-auto w-full"}
									>
										<p className="font-normal opacity-50 text-sm whitespace-pre">
											{project.path}
										</p>
									</TooltipTriggerIfValid>
									<TooltipContent>{project.path}</TooltipContent>
								</Tooltip>
							</div>
						</TooltipTriggerIfInvalid>
						<TooltipPortal>
							<TooltipContent>
								{removed
									? tc("projects:tooltip:no directory")
									: tc("projects:tooltip:invalid project")}
							</TooltipContent>
						</TooltipPortal>
					</Tooltip>
				</td>
				<td className={`${cellClass} w-[8em] min-w-[8em]`}>
					<div className="flex flex-row gap-2">
						<div className="flex items-center">
							{projectTypeKind === "avatars" ? (
								<CircleUserRound className={typeIconClass} />
							) : projectTypeKind === "worlds" ? (
								<Globe className={typeIconClass} />
							) : (
								<CircleHelp className={typeIconClass} />
							)}
						</div>
						<div className="flex flex-col justify-center">
							<p className="font-normal">{displayType}</p>
							{isLegacy && (
								<p className="font-normal opacity-50 dark:opacity-80 text-sm text-destructive">
									{tc("projects:type:legacy")}
								</p>
							)}
						</div>
					</div>
				</td>
				<td className={noGrowCellClass}>
					<p className="font-normal">{project.unity}</p>
				</td>
				<td className={noGrowCellClass}>
					<Tooltip>
						<TooltipTrigger>
							<time dateTime={lastModified.toISOString()}>
								<time className="font-normal">
									{formatDateOffset(project.last_modified)}
								</time>
							</time>
						</TooltipTrigger>
						<TooltipPortal>
							<TooltipContent>
								{dateToString(project.last_modified)}
							</TooltipContent>
						</TooltipPortal>
					</Tooltip>
				</td>
				<td className={noGrowCellClass}>
					<div className="flex flex-row gap-2 max-w-min">
						<ButtonDisabledIfInvalid asChild>
							<OpenUnityButton
								projectPath={project.path}
								unityVersion={project.unity}
								unityRevision={project.unity_revision}
							/>
						</ButtonDisabledIfInvalid>
						<ManageOrMigrateButton project={project} />
						<ButtonDisabledIfInvalid
							onClick={async () => {
								try {
									await openSingleDialog(BackupProjectDialog, {
										projectPath: project.path,
									});
								} catch (e) {
									console.error(e);
									toastThrownError(e);
								}
							}}
							variant={"success"}
						>
							{tc("projects:backup")}
						</ButtonDisabledIfInvalid>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="ghost"
									size={"icon"}
									className={
										"hover:bg-primary/10 text-primary hover:text-primary"
									}
								>
									<Ellipsis className={"size-5"} />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent>
								<DropdownMenuItem
									onClick={openProjectFolder}
									disabled={removed || loading}
								>
									{tc("projects:menuitem:open directory")}
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={onCopyProject}
									disabled={removed || !(is_valid ?? true)}
								>
									{tc("projects:menuitem:copy project")}
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={() =>
										openSingleDialog(RemoveProjectDialog, { project })
									}
									disabled={loading}
									className={"text-destructive focus:text-destructive"}
								>
									{tc("projects:remove project")}
								</DropdownMenuItem>
							</DropdownMenuContent>
						</DropdownMenu>
					</div>
				</td>
			</tr>
		</ProjectContext.Provider>
	);
}

export function ManageOrMigrateButton({ project }: { project: TauriProject }) {
	const navigate = useNavigate();

	if (compareUnityVersionString(project.unity, "2018.0.0f0") < 0) {
		// No UPM is supported in unity 2017 or older
		return (
			<Tooltip>
				<TooltipTriggerIfValid asChild>
					<ButtonDisabledIfInvalid variant="success" disabled>
						{tc("projects:button:manage")}
					</ButtonDisabledIfInvalid>
				</TooltipTriggerIfValid>
				<TooltipContent>
					{tc("projects:tooltip:no upm in unity")}
				</TooltipContent>
			</Tooltip>
		);
	}

	switch (project.project_type) {
		case "LegacySdk2":
			return (
				<Tooltip>
					<TooltipTriggerIfValid asChild>
						<ButtonDisabledIfInvalid variant="success" disabled>
							{tc("projects:button:migrate")}
						</ButtonDisabledIfInvalid>
					</TooltipTriggerIfValid>
					<TooltipContent>
						{tc("projects:tooltip:sdk2 migration hint")}
					</TooltipContent>
				</Tooltip>
			);
		case "LegacyWorlds":
		case "LegacyAvatars":
			return (
				<ButtonDisabledIfInvalid
					variant={"success"}
					onClick={() => void migrateVpm(project.path)}
				>
					{tc("projects:button:migrate")}
				</ButtonDisabledIfInvalid>
			);
		case "UpmWorlds":
		case "UpmAvatars":
		case "UpmStarter":
			return (
				<Tooltip>
					<TooltipTriggerIfValid asChild>
						<ButtonDisabledIfInvalid variant="info" disabled>
							{tc("projects:button:manage")}
						</ButtonDisabledIfInvalid>
					</TooltipTriggerIfValid>
					<TooltipContent>
						{tc("projects:tooltip:git-vcc not supported")}
					</TooltipContent>
				</Tooltip>
			);
		case "Unknown":
		case "Worlds":
		case "Avatars":
		case "VpmStarter":
			return (
				<ButtonDisabledIfInvalid
					onClick={() =>
						navigate({
							to: "/projects/manage",
							search: { projectPath: project.path },
						})
					}
					variant="info"
				>
					{tc("projects:button:manage")}
				</ButtonDisabledIfInvalid>
			);
	}
}

type MigrationProjectBackupType = "none" | "copy" | "backupArchive";

async function migrateVpm(projectPath: string) {
	if (await commands.projectIsUnityLaunching(projectPath)) {
		toastError(tt("projects:toast:close unity before migration"));
		return;
	}

	using dialog = showDialog();

	const backupType = await dialog.ask(ConfirmVpmMigrationDialog, {});
	if (backupType == null) return "";

	let migrateProjectPath: string;
	switch (backupType) {
		case "none":
			migrateProjectPath = projectPath;
			break;
		case "copy": {
			migrateProjectPath = await dialog.ask(MigrationCopyingDialog, {
				header: tc("projects:dialog:vpm migrate header"),
				projectPath,
			});
			break;
		}
		case "backupArchive": {
			const result = await dialog.ask(BackupProjectDialog, {
				projectPath,
			});
			if (result === "cancelled") {
				return;
			}
			migrateProjectPath = projectPath;
			break;
		}
		default:
			assertNever(backupType);
	}
	dialog.replace(<VpmMigrationUpdating />);
	await commands.projectMigrateProjectToVpm(migrateProjectPath);
	toastSuccess(tt("projects:toast:project migrated"));

	await queryClient.invalidateQueries({
		queryKey: ["environmentProjects"],
	});

	router.navigate({
		to: "/projects/manage",
		search: {
			projectPath: migrateProjectPath,
		},
	});
}

function ConfirmVpmMigrationDialog({
	dialog,
}: {
	dialog: DialogContext<MigrationProjectBackupType | null>;
}) {
	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:dialog:vpm migrate header")}</DialogTitle>
			<DialogDescription>
				<p>{tc("projects:dialog:vpm migrate description")}</p>
			</DialogDescription>
			<DialogFooter className={"gap-1"}>
				<Button onClick={() => dialog.close(null)}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close("backupArchive")}>
					{tc("projects:button:backup and migrate")}
				</Button>
				<Button onClick={() => dialog.close("copy")}>
					{tc("projects:button:migrate copy")}
				</Button>
				<Button onClick={() => dialog.close("none")} variant={"destructive"}>
					{tc("projects:button:migrate in-place")}
				</Button>
			</DialogFooter>
		</div>
	);
}

function VpmMigrationUpdating() {
	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:dialog:vpm migrate header")}</DialogTitle>
			<DialogDescription>
				<p>{tc("projects:migrating...")}</p>
			</DialogDescription>
		</div>
	);
}

// region utilities

export const ProjectContext = React.createContext<{
	removed: boolean;
	is_valid: boolean | null;
	loading: boolean;
}>({
	removed: false,
	is_valid: null,
	loading: false,
});

export const ButtonDisabledIfInvalid = function RemovedButton(
	props: React.ComponentProps<typeof Button>,
) {
	const rowContext = useContext(ProjectContext);
	if (rowContext.removed || !(rowContext.is_valid ?? true)) {
		return (
			<Tooltip>
				<TooltipTrigger asChild>
					<Button
						{...props}
						className={`disabled:pointer-events-auto ${props.className}`}
						disabled
					/>
				</TooltipTrigger>
				<TooltipPortal>
					<TooltipContent>
						{rowContext.removed
							? tc("projects:tooltip:no directory")
							: tc("projects:tooltip:invalid project")}
					</TooltipContent>
				</TooltipPortal>
			</Tooltip>
		);
	} else {
		return (
			<Button
				{...props}
				className={`disabled:pointer-events-auto ${props.className}`}
				disabled={props.disabled || rowContext.loading || rowContext.removed}
			/>
		);
	}
};

export const TooltipTriggerIfInvalid = ({
	children,
	...props
}: ComponentProps<typeof TooltipTrigger>) => {
	const rowContext = useContext(ProjectContext);
	if (rowContext.removed || !(rowContext.is_valid ?? true)) {
		return <TooltipTrigger {...props}>{children}</TooltipTrigger>;
	} else {
		return children;
	}
};

export const TooltipTriggerIfValid = ({
	children,
	...props
}: ComponentProps<typeof TooltipTrigger>) => {
	const rowContext = useContext(ProjectContext);
	if (rowContext.removed || !(rowContext.is_valid ?? true)) {
		return children;
	} else {
		return <TooltipTrigger {...props}>{children}</TooltipTrigger>;
	}
};

export function FavoriteToggleButton({
	project,
	disabled,
	onToggle,
	className,
}: {
	project: { favorite: boolean };
	disabled?: boolean;
	onToggle: () => void;
	className?: string;
}) {
	if (disabled) return null;

	return (
		<Star
			strokeWidth={project.favorite ? 1.5 : 3}
			className={cn(
				"size-4 transition-colors cursor-pointer",
				project.favorite ? "text-foreground" : "text-foreground/30",
				!project.favorite && "opacity-0 group-hover:opacity-100",
				"hover:text-foreground",
				className,
			)}
			fill={project.favorite ? "currentColor" : "none"}
			onClick={() => {
				if (!disabled) {
					onToggle();
				}
			}}
		/>
	);
}

export function getProjectDisplayInfo(project: TauriProject) {
	const projectTypeKind = ProjectDisplayType[project.project_type] ?? "unknown";
	const displayType = tc(`projects:type:${projectTypeKind}`);
	const isLegacy = LegacyProjectTypes.includes(project.project_type);
	const lastModified = new Date(project.last_modified);

	return {
		projectTypeKind,
		displayType,
		isLegacy,
		lastModified,
	};
}

export function useSetProjectFavoriteMutation() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (project: Pick<TauriProject, "path" | "favorite">) =>
			commands.environmentSetFavoriteProject(project.path, project.favorite),

		onMutate: async (project) => {
			await queryClient.cancelQueries(environmentProjects);

			const previousData = queryClient.getQueryData<TauriProject[]>(
				environmentProjects.queryKey,
			);

			if (previousData !== undefined) {
				queryClient.setQueryData<TauriProject[]>(
					environmentProjects.queryKey,
					previousData.map((v) =>
						v.path === project.path ? { ...v, favorite: project.favorite } : v,
					),
				);
			}

			return previousData;
		},

		onError: (error, _, context) => {
			console.error("Error migrating project", error);
			toastThrownError(error);
			if (context) {
				queryClient.setQueryData(environmentProjects.queryKey, context);
			}
		},
	});
}

// endregion
