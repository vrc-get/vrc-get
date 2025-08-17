import { CircleHelp, CircleUserRound, Ellipsis, Globe } from "lucide-react";
import {
	ButtonDisabledIfInvalid,
	getProjectDisplayInfo,
	ManageOrMigrateButton,
	ProjectContext,
	TooltipTriggerIfInvalid,
	TooltipTriggerIfValid,
	useSetProjectFavoriteMutation,
} from "@/app/_main/projects/-project-row";
import { copyProject } from "@/app/_main/projects/manage/-copy-project";
import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { FavoriteStarToggleButton } from "@/components/FavoriteStarButton";
import { OpenUnityButton } from "@/components/OpenUnityButton";
import { RemoveProjectDialog } from "@/components/RemoveProjectDialog";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
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
import type { TauriProject } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { dateToString, formatDateOffset } from "@/lib/dateToString";
import { openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";

export function ProjectGridItem({
	project,
	loading,
}: {
	project: TauriProject;
	loading?: boolean;
}) {
	const setProjectFavorite = useSetProjectFavoriteMutation();

	const typeIconClass = "w-5 h-5";

	const { projectTypeKind, displayType, isLegacy, lastModified } =
		getProjectDisplayInfo(project);

	const removed = !project.is_exists;
	const is_valid = project.is_valid;

	return (
		<ProjectContext.Provider
			value={{ removed, is_valid, loading: Boolean(loading) }}
		>
			<Card className="relative p-4 bg-card flex flex-col gap-2 group compact:py-2 compact:pr-2 compact:gap-1">
				<div className={"absolute top-2 right-2 gap-2 flex"}>
					<div className="relative content-center">
						<FavoriteStarToggleButton
							favorite={project.favorite}
							disabled={removed || loading}
							onToggle={() =>
								setProjectFavorite.mutate({
									...project,
									favorite: !project.favorite,
								})
							}
						/>
					</div>
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant="ghost" size="icon">
								<Ellipsis className="size-5" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent>
							<DropdownMenuItem
								onClick={() =>
									commands.utilOpen(project.path, "ErrorIfNotExists")
								}
								disabled={!project.is_exists || loading}
							>
								{tc("projects:menuitem:open directory")}
							</DropdownMenuItem>
							<DropdownMenuItem
								onClick={async () => {
									try {
										await copyProject(project.path);
									} catch (e) {
										console.error(e);
										toastThrownError(e);
									}
								}}
								disabled={!project.is_valid}
							>
								{tc("projects:menuitem:copy project")}
							</DropdownMenuItem>
							<DropdownMenuItem
								onClick={() =>
									openSingleDialog(RemoveProjectDialog, { project })
								}
								disabled={loading}
								className="text-destructive focus:text-destructive"
							>
								{tc("projects:remove project")}
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>

				<Tooltip>
					<TooltipTriggerIfInvalid
						className={"text-left select-text cursor-auto w-full"}
					>
						<div className="flex flex-col">
							<Tooltip>
								<TooltipTriggerIfValid
									className={"text-left select-text cursor-auto w-full"}
								>
									<p className="font-normal whitespace-pre overflow-ellipsis overflow-hidden">
										{project.name}
									</p>
									<p className="font-normal opacity-50 text-sm whitespace-pre overflow-ellipsis overflow-hidden compact:hidden">
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

					<p className="text-sm flex flex-col justify-center">·</p>

					<div className="flex flex-col justify-center">
						<p className={"text-sm"}>{project.unity}</p>
					</div>
				</div>

				<div className="text-xs text-muted-foreground">
					{tc("general:last modified")}:{" "}
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
				</div>

				<div className="mt-2 flex flex-wrap gap-2 justify-end">
					<ButtonDisabledIfInvalid asChild>
						<OpenUnityButton
							projectPath={project.path}
							unityVersion={project.unity}
							unityRevision={project.unity_revision}
						/>
					</ButtonDisabledIfInvalid>
					<ManageOrMigrateButton project={project} />
					<ButtonDisabledIfInvalid
						onClick={() =>
							openSingleDialog(BackupProjectDialog, {
								projectPath: project.path,
							})
						}
						variant="success"
					>
						{tc("projects:backup")}
					</ButtonDisabledIfInvalid>
				</div>
			</Card>
		</ProjectContext.Provider>
	);
}
