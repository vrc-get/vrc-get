import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { assertNever } from "@/lib/assert-never";
import {
	type TauriProject,
	environmentRemoveProject,
	environmentRemoveProjectByPath,
} from "@/lib/bindings";
import { tc, tt } from "@/lib/i18n";
import { toastSuccess } from "@/lib/toast";
import { type ReactNode, useState } from "react";

// string if remove project by path
type Project =
	| TauriProject
	| {
			path: string;
			name: string;
			is_exists: boolean;
	  };

type State =
	| {
			type: "idle";
	  }
	| {
			type: "confirm";
			project: Project;
	  }
	| {
			type: "removing";
	  };

type Params = {
	onRemoving?: () => void;
	onRemoved?: () => void;
};

type Result = {
	startRemove: (project: Project) => void;
	dialog: ReactNode;
};

export function useRemoveProjectModal({ onRemoved }: Params): Result {
	const [state, setState] = useState<State>({ type: "idle" });

	const cancel = () => setState({ type: "idle" });
	const startRemove = (project: Project) =>
		setState({ type: "confirm", project });

	let dialog: ReactNode = null;

	switch (state.type) {
		case "idle":
			break;
		case "confirm": {
			const project = state.project;

			const removeProjectButton = async (directory: boolean) => {
				setState({ type: "removing" });
				try {
					if ("list_version" in project) {
						await environmentRemoveProject(
							project.list_version,
							project.index,
							directory,
						);
					} else {
						await environmentRemoveProjectByPath(project.path, directory);
					}
					toastSuccess(tt("projects:toast:project removed"));
					setState({ type: "idle" });
				} finally {
					onRemoved?.();
				}
			};

			dialog = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:remove project")}</DialogTitle>
					<DialogDescription>
						<p className={"font-normal"}>
							{tc("projects:dialog:warn removing project", {
								name: project.name,
							})}
						</p>
					</DialogDescription>
					<DialogFooter>
						<Button onClick={cancel} className="mr-1">
							{tc("general:button:cancel")}
						</Button>
						<Button
							onClick={() => removeProjectButton(false)}
							className="mr-1 px-2"
						>
							{tc("projects:button:remove from list")}
						</Button>
						<Button
							onClick={() => removeProjectButton(true)}
							variant={"destructive"}
							className="px-2"
							disabled={!project.is_exists}
						>
							{tc("projects:button:remove directory")}
						</Button>
					</DialogFooter>
				</DialogOpen>
			);
			break;
		}
		case "removing":
			dialog = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:remove project")}</DialogTitle>
					<DialogDescription>
						{tc("projects:dialog:removing...")}
					</DialogDescription>
					<DialogFooter>
						<Button className="mr-1" disabled>
							{tc("general:button:cancel")}
						</Button>
						<Button className="mr-1 px-2" disabled>
							{tc("projects:button:remove from list")}
						</Button>
						<Button variant={"destructive"} className="px-2" disabled>
							{tc("projects:button:remove directory")}
						</Button>
					</DialogFooter>
				</DialogOpen>
			);
			break;
		default:
			assertNever(state);
	}

	return { startRemove, dialog };
}
