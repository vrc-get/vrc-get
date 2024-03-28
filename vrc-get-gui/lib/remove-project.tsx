import React, {ReactNode, useState} from "react";
import {Button, Dialog, DialogBody, DialogFooter, DialogHeader} from "@material-tailwind/react";
import {nop} from "@/lib/nop";
import {Trans, useTranslation} from "react-i18next";
import {environmentRemoveProject, environmentRemoveProjectByPath, TauriProject} from "@/lib/bindings";
import {toastSuccess} from "@/lib/toast";

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
	const {t} = useTranslation();

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
						await environmentRemoveProject(project.list_version, project.index, directory);
					} else {
						await environmentRemoveProjectByPath(project.path, directory);
					}
					toastSuccess("Project removed successfully");
					setState({type: 'idle'});
				} finally {
					onRemoved?.();
				}
			}

			dialog = (
				<Dialog open handler={nop} className={'whitespace-normal'}>
					<DialogHeader>{t("remove project")}</DialogHeader>
					<DialogBody>
						<Trans i18nKey={"you're about to remove the project <strong>{{name}}</strong>"}
									 values={{name: project.name}}
									 components={{strong: <strong/>}}
						/>
					</DialogBody>
					<DialogFooter>
						<Button onClick={cancel} className="mr-1">{t("cancel")}</Button>
						<Button onClick={() => removeProjectButton(false)} className="mr-1 px-2">
							{t("remove from the list")}
						</Button>
						<Button onClick={() => removeProjectButton(true)} color={"red"} className="px-2"
										disabled={!project.is_exists}>
							{t("remove the directory")}
						</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		case "removing":
			dialog = (
				<Dialog open handler={nop} className={'whitespace-normal'}>
					<DialogHeader>{t("remove project")}</DialogHeader>
					<DialogBody>
						<Trans i18nKey={"Removing the project..."}/>
					</DialogBody>
					<DialogFooter>
						<Button className="mr-1" disabled>{t("cancel")}</Button>
						<Button className="mr-1 px-2" disabled>
							{t("remove from the list")}
						</Button>
						<Button color={"red"} className="px-2" disabled>
							{t("remove the directory")}
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
