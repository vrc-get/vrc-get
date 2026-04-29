import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useLocation, useRouter } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import { commands } from "@/lib/bindings";
import type { DialogContext } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { nameFromPath } from "@/lib/os";
import { toastSuccess, toastThrownError } from "@/lib/toast";

type Project = {
	path: string;
	is_exists: boolean;
};

export function RemoveProjectDialog({
	project,
	dialog,
}: {
	project: Project;
	dialog: DialogContext<boolean>;
}) {
	const queryClient = useQueryClient();
	const router = useRouter();
	const location = useLocation();

	const removeProject = useMutation({
		mutationFn: async ({
			project,
			removeDir,
		}: {
			project: Project;
			removeDir: boolean;
		}) => {
			await commands.environmentRemoveProjectByPath(project.path, removeDir);
		},
		onSuccess: () => {
			dialog.close(true);
			toastSuccess(tt("projects:toast:project removed"));
		},
		onError: (e) => {
			console.error(e);
			dialog.close(false);
			toastThrownError(e);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["environmentProjects"],
			});
			if (
				location.pathname === "/projects/manage" &&
				location.search.projectPath === project.path
			) {
				router.history.back();
			}
		},
	});

	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:remove project")}</DialogTitle>
			<div>
				{removeProject.isPending ? (
					<p className={"font-normal"}>{tc("projects:dialog:removing...")}</p>
				) : (
					<p className={"font-normal"}>
						{tc("projects:dialog:warn removing project", {
							name: nameFromPath(project.path),
						})}
					</p>
				)}
			</div>
			<DialogFooter className={"flex gap-2"}>
				<Button
					onClick={() => dialog.close(false)}
					disabled={removeProject.isPending}
				>
					{tc("general:button:cancel")}
				</Button>
				<Button
					onClick={() => removeProject.mutate({ project, removeDir: false })}
					className="px-2"
					disabled={removeProject.isPending}
				>
					{tc("projects:button:remove from list")}
				</Button>
				<Button
					onClick={() => removeProject.mutate({ project, removeDir: true })}
					variant={"destructive"}
					className="px-2"
					disabled={!project.is_exists || removeProject.isPending}
				>
					{tc("projects:button:remove directory")}
				</Button>
			</DialogFooter>
		</div>
	);
}
