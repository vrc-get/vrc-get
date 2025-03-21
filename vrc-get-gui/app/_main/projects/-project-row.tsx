import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { RemoveProjectDialog } from "@/components/RemoveProjectDialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { Progress } from "@/components/ui/progress";
import {
	Tooltip,
	TooltipContent,
	TooltipPortal,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriCopyProjectForMigrationProgress,
	TauriProject,
	TauriProjectType,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { openSingleDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { openUnity } from "@/lib/open-unity";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useNavigate } from "@tanstack/react-router";
import {
	CircleHelp,
	CircleUserRound,
	Ellipsis,
	Globe,
	Star,
} from "lucide-react";
import React, {
	type ComponentProps,
	forwardRef,
	useContext,
	useState,
} from "react";

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

export function ProjectRow({
	project,
	loading,
	refresh,
}: {
	project: TauriProject;
	loading?: boolean;
	refresh?: () => void;
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

	const onToggleFavorite = async () => {
		try {
			await commands.environmentSetFavoriteProject(
				project.list_version,
				project.index,
				!project.favorite,
			);
			refresh?.();
		} catch (e) {
			console.error("Error migrating project", e);
			toastThrownError(e);
		}
	};

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
							onCheckedChange={onToggleFavorite}
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
						<ButtonDisabledIfRemoved
							onClick={() =>
								openUnity(project.path, project.unity, project.unity_revision)
							}
						>
							{tc("projects:button:open unity")}
						</ButtonDisabledIfRemoved>
						<ManageOrMigrateButton project={project} refresh={refresh} />
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
	refresh,
}: {
	project: TauriProject;
	refresh?: () => void;
}) {
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
			return <MigrateButton project={project} refresh={refresh} />;
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

function MigrateButton({
	project,
	refresh,
}: {
	project: TauriProject;
	refresh?: () => void;
}) {
	type MigrateState =
		| {
				type: "normal";
		  }
		| {
				type: "migrateVpm:confirm";
		  }
		| {
				type: "migrateVpm:copyingProject";
				progress: TauriCopyProjectForMigrationProgress;
		  }
		| {
				type: "migrateVpm:updating";
		  };

	type ProjectBackupType = "none" | "copy" | "backupArchive";

	const [dialogStatus, setDialogStatus] = useState<MigrateState>({
		type: "normal",
	});

	const startMigrateVpm = async () => {
		if (await commands.projectIsUnityLaunching(project.path)) {
			toastError(tt("projects:toast:close unity before migration"));
			return;
		}
		setDialogStatus({ type: "migrateVpm:confirm" });
	};

	const doMigrateVpm = async (backupType: ProjectBackupType) => {
		setDialogStatus({ type: "normal" });
		try {
			let migrateProjectPath: string;
			switch (backupType) {
				case "none":
					migrateProjectPath = project.path;
					break;
				case "copy": {
					setDialogStatus({
						type: "migrateVpm:copyingProject",
						progress: {
							proceed: 0,
							total: 1,
							last_proceed: "Collecting files...",
						},
					});
					const [, promise] = callAsyncCommand(
						commands.environmentCopyProjectForMigration,
						[project.path],
						(progress) => {
							setDialogStatus((prev) => {
								if (prev.type !== "migrateVpm:copyingProject") return prev;
								if (prev.progress.proceed > progress.proceed) return prev;
								return { ...prev, progress };
							});
						},
					);
					migrateProjectPath = await promise;
					break;
				}
				case "backupArchive": {
					const result = await openSingleDialog(BackupProjectDialog, {
						projectPath: project.path,
					});
					if (result === "cancelled") {
						return;
					}
					migrateProjectPath = project.path;
					break;
				}
				default:
					assertNever(backupType);
			}
			setDialogStatus({ type: "migrateVpm:updating" });
			await commands.projectMigrateProjectToVpm(migrateProjectPath);
			setDialogStatus({ type: "normal" });
			toastSuccess(tt("projects:toast:project migrated"));
			refresh?.();
		} catch (e) {
			console.error("Error migrating project", e);
			setDialogStatus({ type: "normal" });
			toastThrownError(e);
		}
	};

	let dialogContent: React.ReactNode = null;
	switch (dialogStatus.type) {
		case "migrateVpm:confirm":
			dialogContent = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:dialog:vpm migrate header")}</DialogTitle>
					<DialogDescription>
						<p>{tc("projects:dialog:vpm migrate description")}</p>
					</DialogDescription>
					<DialogFooter className={"gap-1"}>
						<Button onClick={() => setDialogStatus({ type: "normal" })}>
							{tc("general:button:cancel")}
						</Button>
						<Button onClick={() => doMigrateVpm("backupArchive")}>
							{tc("projects:button:backup and migrate")}
						</Button>
						<Button onClick={() => doMigrateVpm("copy")}>
							{tc("projects:button:migrate copy")}
						</Button>
						<Button
							onClick={() => doMigrateVpm("none")}
							variant={"destructive"}
						>
							{tc("projects:button:migrate in-place")}
						</Button>
					</DialogFooter>
				</DialogOpen>
			);
			break;
		case "migrateVpm:copyingProject":
			dialogContent = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:dialog:vpm migrate header")}</DialogTitle>
					<DialogDescription>
						<p>{tc("projects:pre-migrate copying...")}</p>
						<p>
							{tc("projects:dialog:proceed k/n", {
								count: dialogStatus.progress.proceed,
								total: dialogStatus.progress.total,
							})}
						</p>
						<Progress
							value={dialogStatus.progress.proceed}
							max={dialogStatus.progress.total}
						/>
					</DialogDescription>
				</DialogOpen>
			);
			break;
		case "migrateVpm:updating":
			dialogContent = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:dialog:vpm migrate header")}</DialogTitle>
					<DialogDescription>
						<p>{tc("projects:migrating...")}</p>
					</DialogDescription>
				</DialogOpen>
			);
			break;
	}

	return (
		<>
			<ButtonDisabledIfRemoved variant={"success"} onClick={startMigrateVpm}>
				{tc("projects:button:migrate")}
			</ButtonDisabledIfRemoved>
			{dialogContent}
		</>
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

const ButtonDisabledIfRemoved = forwardRef<
	HTMLButtonElement,
	React.ComponentProps<typeof Button>
>(function RemovedButton(props, ref) {
	const rowContext = useContext(ProjectRowContext);
	if (rowContext.removed) {
		return (
			<Tooltip>
				<TooltipTrigger asChild>
					<Button
						{...props}
						className={`disabled:pointer-events-auto ${props.className}`}
						disabled
						ref={ref}
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
				ref={ref}
			/>
		);
	}
});

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
