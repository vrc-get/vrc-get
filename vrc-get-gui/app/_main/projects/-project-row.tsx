import { copyProject } from "@/app/_main/projects/manage/-copy-project";
import { MigrationCopyingDialog } from "@/app/_main/projects/manage/-unity-migration";
import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { OpenUnityButton } from "@/components/OpenUnityButton";
import { RemoveProjectDialog } from "@/components/RemoveProjectDialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { type DialogContext, openSingleDialog, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { router } from "@/lib/main";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { compareUnityVersionString } from "@/lib/version";
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

const ProjectDisplayType: Record<
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

const LegacyProjectTypes = [
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

	const projectTypeKind = ProjectDisplayType[project.project_type] ?? "unknown";
	const displayType = tc(`projects:type:${projectTypeKind}`);
	const isLegacy = LegacyProjectTypes.includes(project.project_type);
	const lastModified = new Date(project.last_modified);
	const lastModifiedHumanReadable = `${lastModified.getFullYear().toString().padStart(4, "0")}-${(lastModified.getMonth() + 1).toString().padStart(2, "0")}-${lastModified.getDate().toString().padStart(2, "0")} ${lastModified.getHours().toString().padStart(2, "0")}:${lastModified.getMinutes().toString().padStart(2, "0")}:${lastModified.getSeconds().toString().padStart(2, "0")}`;

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

	const queryClient = useQueryClient();
	const setProjectFavorite = useMutation({
		mutationFn: (
			project: Pick<TauriProject, "list_version" | "index" | "favorite">,
		) =>
			commands.environmentSetFavoriteProject(
				project.list_version,
				project.index,
				project.favorite,
			),
		onMutate: async (project) => {
			await queryClient.cancelQueries(environmentProjects);
			const data = queryClient.getQueryData(environmentProjects.queryKey);
			if (data !== undefined) {
				queryClient.setQueryData(
					environmentProjects.queryKey,
					data.map((v) =>
						v.list_version === project.list_version && v.index === project.index
							? { ...v, favorite: project.favorite }
							: v,
					),
				);
			}
			return data;
		},
		onError: (e, _, ctx) => {
			console.error("Error migrating project", e);
			toastThrownError(e);
			queryClient.setQueryData(environmentProjects.queryKey, ctx);
		},
	});

	const removed = !project.is_exists;

	return (
		<ProjectRowContext.Provider value={{ removed, loading: Boolean(loading) }}>
			<tr
				className={`even:bg-secondary/30 ${removed || loading ? "opacity-50" : ""}`}
			>
				<td className={`${cellClass} w-3`}>
					<div className={"relative flex"}>
						<Checkbox
							checked={project.favorite}
							onCheckedChange={() =>
								setProjectFavorite.mutate({
									...project,
									favorite: !project.favorite,
								})
							}
							disabled={removed || loading}
							className="before:transition-none border-none text-primary! peer"
						/>
						<span
							className={
								"text-foreground/30 peer-data-[state=checked]:text-background pointer-events-none absolute top-2/4 left-2/4 -translate-y-2/4 -translate-x-2/4"
							}
						>
							<Star strokeWidth={3} className={"size-3"} />
						</span>
					</div>
				</td>
				<td className={`${cellClass} max-w-64 overflow-hidden`}>
					<Tooltip>
						<TooltipTriggerIfRemoved
							className={"text-left select-text cursor-auto w-full"}
						>
							<div className="flex flex-col">
								<Tooltip>
									<TooltipTriggerIfExists
										className={"text-left select-text cursor-auto w-full"}
									>
										<p className="font-normal whitespace-pre">{project.name}</p>
									</TooltipTriggerIfExists>
									<TooltipContent>{project.name}</TooltipContent>
								</Tooltip>
								<Tooltip>
									<TooltipTriggerIfExists
										className={"text-left select-text cursor-auto w-full"}
									>
										<p className="font-normal opacity-50 text-sm whitespace-pre">
											{project.path}
										</p>
									</TooltipTriggerIfExists>
									<TooltipContent>{project.path}</TooltipContent>
								</Tooltip>
							</div>
						</TooltipTriggerIfRemoved>
						<TooltipPortal>
							<TooltipContent>
								{tc("projects:tooltip:no directory")}
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
							<TooltipContent>{lastModifiedHumanReadable}</TooltipContent>
						</TooltipPortal>
					</Tooltip>
				</td>
				<td className={noGrowCellClass}>
					<div className="flex flex-row gap-2 max-w-min">
						<ButtonDisabledIfRemoved asChild>
							<OpenUnityButton
								projectPath={project.path}
								unityVersion={project.unity}
								unityRevision={project.unity_revision}
							/>
						</ButtonDisabledIfRemoved>
						<ManageOrMigrateButton project={project} />
						<ButtonDisabledIfRemoved
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
						</ButtonDisabledIfRemoved>
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
								<DropdownMenuItem onClick={onCopyProject}>
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
		</ProjectRowContext.Provider>
	);
}

function ManageOrMigrateButton({
	project,
}: {
	project: TauriProject;
}) {
	if (compareUnityVersionString(project.unity, "2018.0.0f0") < 0) {
		// No UPM is supported in unity 2017 or older
		return (
			<Tooltip>
				<TooltipTriggerIfExists asChild>
					<ButtonDisabledIfRemoved variant="success" disabled>
						{tc("projects:button:manage")}
					</ButtonDisabledIfRemoved>
				</TooltipTriggerIfExists>
				<TooltipContent>
					{tc("projects:tooltip:no upm in unity")}
				</TooltipContent>
			</Tooltip>
		);
	}

	const navigate = useNavigate();
	switch (project.project_type) {
		case "LegacySdk2":
			return (
				<Tooltip>
					<TooltipTriggerIfExists asChild>
						<ButtonDisabledIfRemoved variant="success" disabled>
							{tc("projects:button:migrate")}
						</ButtonDisabledIfRemoved>
					</TooltipTriggerIfExists>
					<TooltipContent>
						{tc("projects:tooltip:sdk2 migration hint")}
					</TooltipContent>
				</Tooltip>
			);
		case "LegacyWorlds":
		case "LegacyAvatars":
			return (
				<ButtonDisabledIfRemoved
					variant={"success"}
					onClick={() => void migrateVpm(project.path)}
				>
					{tc("projects:button:migrate")}
				</ButtonDisabledIfRemoved>
			);
		case "UpmWorlds":
		case "UpmAvatars":
		case "UpmStarter":
			return (
				<Tooltip>
					<TooltipTriggerIfExists asChild>
						<ButtonDisabledIfRemoved variant="info" disabled>
							{tc("projects:button:manage")}
						</ButtonDisabledIfRemoved>
					</TooltipTriggerIfExists>
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
				<ButtonDisabledIfRemoved
					onClick={() =>
						navigate({
							to: "/projects/manage",
							search: { projectPath: project.path },
						})
					}
					variant="info"
				>
					{tc("projects:button:manage")}
				</ButtonDisabledIfRemoved>
			);
	}
}

type MigrationProjectBackupType = "none" | "copy" | "backupArchive";

async function migrateVpm(projectPath: string) {
	if (await commands.projectIsUnityLaunching(projectPath)) {
		toastError(tt("projects:toast:close unity before migration"));
		return;
	}

	using dialog = showDialog(null);

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

const ProjectRowContext = React.createContext<{
	removed: boolean;
	loading: boolean;
}>({
	removed: false,
	loading: false,
});

const ButtonDisabledIfRemoved = function RemovedButton(
	props: React.ComponentProps<typeof Button>,
) {
	const rowContext = useContext(ProjectRowContext);
	if (rowContext.removed) {
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
					<TooltipContent>{tt("projects:tooltip:no directory")}</TooltipContent>
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

const TooltipTriggerIfRemoved = ({
	children,
	...props
}: ComponentProps<typeof TooltipTrigger>) => {
	const rowContext = useContext(ProjectRowContext);
	if (rowContext.removed) {
		return <TooltipTrigger {...props}>{children}</TooltipTrigger>;
	} else {
		return children;
	}
};

const TooltipTriggerIfExists = ({
	children,
	...props
}: ComponentProps<typeof TooltipTrigger>) => {
	const rowContext = useContext(ProjectRowContext);
	if (rowContext.removed) {
		return children;
	} else {
		return <TooltipTrigger {...props}>{children}</TooltipTrigger>;
	}
};

function formatDateOffset(date: number): React.ReactNode {
	const now = Date.now();
	const diff = now - date;

	const PER_SECOND = 1000;
	const PER_MINUTE = 60 * PER_SECOND;
	const PER_HOUR = 60 * PER_MINUTE;
	const PER_DAY = 24 * PER_HOUR;
	const PER_WEEK = 7 * PER_DAY;
	const PER_MONTH = 30 * PER_DAY;
	const PER_YEAR = 365 * PER_DAY;

	const diffAbs = Math.abs(diff);

	if (diffAbs < PER_MINUTE) return tc("projects:last modified:moments");
	if (diffAbs < PER_HOUR)
		return tc("projects:last modified:minutes", {
			count: Math.floor(diff / PER_MINUTE),
		});
	if (diffAbs < PER_DAY)
		return tc("projects:last modified:hours", {
			count: Math.floor(diff / PER_HOUR),
		});
	if (diffAbs < PER_WEEK)
		return tc("projects:last modified:days", {
			count: Math.floor(diff / PER_DAY),
		});
	if (diffAbs < PER_MONTH)
		return tc("projects:last modified:weeks", {
			count: Math.floor(diff / PER_WEEK),
		});
	if (diffAbs < PER_YEAR)
		return tc("projects:last modified:months", {
			count: Math.floor(diff / PER_MONTH),
		});

	return tc("projects:last modified:years", {
		count: Math.floor(diff / PER_YEAR),
	});
}

// endregion
