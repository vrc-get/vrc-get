import {useMutation, useQueryClient} from "@tanstack/react-query";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {commands, TauriProject} from "@/lib/bindings";
import type { DialogContext } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { nameFromPath } from "@/lib/os";
import { toastSuccess, toastThrownError } from "@/lib/toast";

type Project = {
	path: string;
	is_exists: boolean;
	name: string;
	display_name: string | null;
};

export function SetProjectDisplayNameDialog({
																			project,
																			dialog,
																		}: {
	project: Project;
	dialog: DialogContext<boolean>;
}) {
	const queryClient = useQueryClient();
	const [name, setName] = useState(project.display_name || project.name);

	const changeDisplayName = useMutation({
		mutationFn: async ({
												 project,
												 newName,
												 clear,
											 }: {
			project: Project;
			newName: string;
			clear: boolean;
		}) => {
			newName = newName.trim();

			if (clear || newName === '' || newName === project.name) {
				commands.projectClearDisplayName(project.path);
			}
			else {
				commands.projectSetDisplayName(project.path, newName);
			}
		},
		onSuccess: () => {
			dialog.close(true);
			toastSuccess(tt("projects:toast:project display name changed"));
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
		},
	});

	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:set project display name")}</DialogTitle>
			<div>
				<p className={"font-normal"}>
					{tc("projects:warn set project display name")}
				</p>
				<div className="grid gap-2 pt-4">
					<Label htmlFor="name">{tc("general:name")}</Label>
					<Input
						id="name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						placeholder={nameFromPath(project.path)}
						disabled={changeDisplayName.isPending}
						autoFocus
					/>
				</div>
			</div>
			<DialogFooter className={"flex gap-2"}>
				<Button
					onClick={() => dialog.close(false)}
					disabled={changeDisplayName.isPending}
				>
					{tc("general:button:cancel")}
				</Button>
				<Button
					variant="secondary"
					onClick={() =>
						changeDisplayName.mutate({ project, newName: "", clear: true })
					}
					disabled={changeDisplayName.isPending}
				>
					{tc("general:button:reset")}
				</Button>
				<Button
					onClick={() =>
						changeDisplayName.mutate({ project, newName: name, clear: false })
					}
					disabled={(project.display_name || project.name) === name || changeDisplayName.isPending || !name.trim()}
				>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</div>
	);
}
