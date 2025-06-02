import {
	ButtonDisabledIfInvalid,
	FavoriteToggleButton,
	LegacyProjectTypes,
	ManageOrMigrateButton,
	ProjectContext,
	ProjectDisplayType,
	TooltipTriggerIfInvalid,
	TooltipTriggerIfValid,
	formatDateOffset,
} from "@/app/_main/projects/-project-row";
import { copyProject } from "@/app/_main/projects/manage/-copy-project";
import { BackupProjectDialog } from "@/components/BackupProjectDialog";
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
import { openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import {
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import { CircleHelp, CircleUserRound, Ellipsis, Globe } from "lucide-react";

const environmentProjects = queryOptions({
	queryKey: ["environmentProjects"],
	queryFn: commands.environmentProjects,
});

export function ProjectGridItem({
	project,
	loading,
}: {
	project: TauriProject;
	loading?: boolean;
}) {
	const queryClient = useQueryClient();

	const setProjectFavorite = useMutation({
		mutationFn: (project: Pick<TauriProject, "path" | "favorite">) =>
			commands.environmentSetFavoriteProject(project.path, project.favorite),
		onMutate: async (project) => {
			await queryClient.cancelQueries(environmentProjects);
			const data = queryClient.getQueryData(environmentProjects.queryKey);
			if (data !== undefined) {
				queryClient.setQueryData(
					environmentProjects.queryKey,
					data.map((v) =>
						v.path === project.path ? { ...v, favorite: project.favorite } : v,
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

	const typeIconClass = "w-5 h-5";

	const projectTypeKind = ProjectDisplayType[project.project_type] ?? "unknown";
	const displayType = tc(`projects:type:${projectTypeKind}`);
	const isLegacy = LegacyProjectTypes.includes(project.project_type);
	const lastModified = new Date(project.last_modified);
	const lastModifiedHumanReadable = `${lastModified.getFullYear().toString().padStart(4, "0")}-${(lastModified.getMonth() + 1).toString().padStart(2, "0")}-${lastModified.getDate().toString().padStart(2, "0")} ${lastModified.getHours().toString().padStart(2, "0")}:${lastModified.getMinutes().toString().padStart(2, "0")}:${lastModified.getSeconds().toString().padStart(2, "0")}`;

	const removed = !project.is_exists;
	const is_valid = project.is_valid;

	return (
		<ProjectContext.Provider
			value={{ removed, is_valid, loading: Boolean(loading) }}
		>
			<Card className="relative p-4 bg-card flex flex-col gap-2 group">
				<div className={"absolute top-2 right-2 gap-2 flex"}>
					<div className="relative content-center">
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
				</div>

				<div className="mt-2 flex flex-wrap gap-2">
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
