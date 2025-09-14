import { useMutation } from "@tanstack/react-query";
import type { NavigateFn } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import { commands, type TauriCopyProjectProgress } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { type DialogContext, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { directoryFromPath, nameFromPath, pathSeparator } from "@/lib/os";
import {
	ProjectNameCheckResult,
	useProjectNameCheck,
} from "@/lib/project-name-check";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";

export async function copyProject(existingPath: string, navigate?: NavigateFn) {
	using dialog = showDialog();
	const newPath = await dialog.ask(CopyProjectNameDialog, {
		projectPath: existingPath,
	});
	if (newPath == null) return; // cancelled
	await dialog.ask(CopyingDialog, {
		projectPath: existingPath,
		newProjectPath: newPath,
	});
	dialog.close();
	toastSuccess(
		tc("projects:toast:successfully copied project", {
			name: nameFromPath(existingPath),
		}),
	);

	await Promise.all([
		queryClient.invalidateQueries({
			queryKey: ["projectDetails", existingPath],
		}),
		queryClient.invalidateQueries({
			queryKey: ["environmentProjects"],
		}),
	]);

	await navigate?.({
		replace: true,
		to: "/projects/manage",
		search: { projectPath: newPath },
	});
}

function CopyProjectNameDialog({
	dialog,
	projectPath,
}: {
	dialog: DialogContext<string | null>;
	projectPath: string;
}) {
	const oldName = nameFromPath(projectPath);
	const [projectNameRaw, setProjectName] = useState(`${oldName}-Copy`);
	const projectName = projectNameRaw.trim();
	const [projectLocation, setProjectLocation] = useState(
		directoryFromPath(projectPath),
	);
	const projectNameCheckState = useProjectNameCheck(
		projectLocation,
		projectName,
	);

	const usePickProjectLocationPath = useMutation({
		mutationFn: () => commands.utilPickDirectory(projectLocation),
		onSuccess: (result) => {
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					setProjectLocation(result.new_path);
					break;
				default:
					assertNever(result);
			}
		},
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
	});

	const createProject = async () => {
		dialog.close(`${projectLocation}${pathSeparator()}${projectName}`);
	};

	const badProjectName = ["AlreadyExists", "InvalidNameForFolderName"].includes(
		projectNameCheckState,
	);

	const canCreateProject =
		projectNameCheckState !== "checking" && !badProjectName;

	return (
		<>
			<DialogTitle>
				{tc("projects:dialog:copy project", { name: oldName })}
			</DialogTitle>
			<div>
				<VStack>
					<Input
						value={projectNameRaw}
						onChange={(e) => setProjectName(e.target.value)}
					/>
					<div className={"flex gap-1 items-center"}>
						<Input className="flex-auto" value={projectLocation} disabled />
						<Button
							className="flex-none px-4"
							onClick={() => usePickProjectLocationPath.mutate()}
						>
							{tc("general:button:select")}
						</Button>
					</div>
					<small className={"whitespace-normal"}>
						{tc(
							"projects:hint:path of creating project",
							{ path: `${projectLocation}${pathSeparator()}${projectName}` },
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
					</small>
					<ProjectNameCheckResult
						projectNameCheckState={projectNameCheckState}
					/>
				</VStack>
			</div>
			<DialogFooter className={"gap-2"}>
				<Button onClick={() => dialog.close(null)}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={createProject} disabled={!canCreateProject}>
					{tc("projects:button:create")}
				</Button>
			</DialogFooter>
		</>
	);
}

export function CopyingDialog({
	projectPath,
	newProjectPath,
	dialog,
}: {
	projectPath: string;
	newProjectPath: string;
	dialog: DialogContext<string>;
}) {
	const oldName = nameFromPath(projectPath);

	const [progress, setProgress] = useState<TauriCopyProjectProgress>({
		proceed: 0,
		total: 1,
		last_proceed: "Collecting files...",
	});

	useEffect(() => {
		const [_, promise] = callAsyncCommand(
			commands.environmentCopyProject,
			[projectPath, newProjectPath],
			(progress) => {
				setProgress((prev) => {
					if (prev.proceed > progress.proceed) return prev;
					return progress;
				});
			},
		);

		promise.then(dialog.close, dialog.error);
	}, [projectPath, newProjectPath, dialog.close, dialog.error]);

	return (
		<>
			<DialogTitle>
				{tc("projects:dialog:copy project", { name: oldName })}
			</DialogTitle>
			<div>
				<p>{tc("projects:dialog:copying...")}</p>
				<p>
					{tc("projects:dialog:proceed k/n", {
						count: progress.proceed,
						total: progress.total,
					})}
				</p>
				<Progress value={progress.proceed} max={progress.total} />
				<p>{tc("projects:do not close")}</p>
			</div>
			<DialogFooter className={"gap-2"}>
				<Button disabled>{tc("general:button:cancel")}</Button>
			</DialogFooter>
		</>
	);
}
