import React, {ReactNode, useState} from "react";
import {Button, Dialog, DialogBody, DialogFooter, DialogHeader, Typography} from "@material-tailwind/react";
import {nop} from "@/lib/nop";
import {environmentRemoveProject, environmentRemoveProjectByPath, TauriProject} from "@/lib/bindings";
import {toastSuccess} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";

// string if remove project by path
type Project = TauriProject | {
	path: string,
	name: string,
	is_exists: boolean,
};

type State = {
	type: 'idle',
} | {
	type: 'confirm',
	project: Project,
} | {
	type: 'removing',
}

type Params = {
	onRemoving?: () => void,
	onRemoved?: () => void,
}

type Result = {
	startRemove: (project: Project) => void,
	dialog: ReactNode,
}

export function useRemoveProjectModal({onRemoved}: Params): Result {
	const [state, setState] = useState<State>({type: 'idle'});

	const cancel = () => setState({type: 'idle'});
	const startRemove = (project: Project) => setState({type: 'confirm', project});

	let dialog: ReactNode = null;

	switch (state.type) {
		case "idle":
			break;
		case "confirm":
			const project = state.project;


			const removeProjectButton = async (directory: boolean) => {
				setState({type: 'removing'});
				try {
					if ('list_version' in project) {
						console.log("remove with index")
						await environmentRemoveProject(project.list_version, project.index, directory);
					} else {
						console.log("remove with path")
						await environmentRemoveProjectByPath(project.path, directory);
					}
					toastSuccess(tt("project removed successfully"));
					setState({type: 'idle'});
				} finally {
					onRemoved?.();
				}
			}

			dialog = (
				<Dialog open handler={nop} className={'whitespace-normal'}>
					<DialogHeader>{tc("remove project")}</DialogHeader>
					<DialogBody>
						<Typography className={"font-normal"}>
							{tc("you're about to remove the project <strong>{{name}}</strong>", {name: project.name})}
						</Typography>
					</DialogBody>
					<DialogFooter>
						<Button onClick={cancel} className="mr-1">{tc("cancel")}</Button>
						<Button onClick={() => removeProjectButton(false)} className="mr-1 px-2">
							{tc("remove from the list")}
						</Button>
						<Button onClick={() => removeProjectButton(true)} color={"red"} className="px-2"
										disabled={!project.is_exists}>
							{tc("remove the directory")}
						</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		case "removing":
			dialog = (
				<Dialog open handler={nop} className={'whitespace-normal'}>
					<DialogHeader>{tc("remove project")}</DialogHeader>
					<DialogBody>
						{tc("removing the project...")}
					</DialogBody>
					<DialogFooter>
						<Button className="mr-1" disabled>{tc("cancel")}</Button>
						<Button className="mr-1 px-2" disabled>
							{tc("remove from the list")}
						</Button>
						<Button color={"red"} className="px-2" disabled>
							{tc("remove the directory")}
						</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		default:
			let _: never = state;
	}

	return {startRemove, dialog}
}
