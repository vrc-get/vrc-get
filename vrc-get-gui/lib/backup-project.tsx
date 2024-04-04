import React, {ReactNode, useState} from "react";
import {Button, Dialog, DialogBody, DialogFooter, DialogHeader, Typography} from "@material-tailwind/react";
import {nop} from "@/lib/nop";
import {
	environmentRemoveProject,
	environmentRemoveProjectByPath,
	projectCreateBackup,
	TauriProject
} from "@/lib/bindings";
import {toastError, toastNormal, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";

// string if remove project by path
type Project = TauriProject | {
	path: string,
	name: string,
};

type State = {
	type: 'idle',
} | {
	type: 'backing-up',
}

type Params = {}

type Result = {
	startBackup: (project: Project) => void,
	dialog: ReactNode,
}

export function useBackupProjectModal(_: Params = {}): Result {
	const [state, setState] = useState<State>({type: 'idle'});

	const cancel = () => toastError("cancel unsupported");
	const startBackup = async (project: Project) => {
		try {
			setState({type: 'backing-up'});
			await projectCreateBackup(project.path);
			toastSuccess("Backup created successfully");
			setState({type: 'idle'});
		} catch (e) {
			console.error("Error creating backup", e);
			setState({type: 'idle'});
			toastThrownError(e);
		}
	};

	let dialog: ReactNode = null;

	switch (state.type) {
		case "idle":
			break;
		case "backing-up":
			dialog = (
				<Dialog open handler={cancel} className={'whitespace-normal'}>
					<DialogHeader>{tc("backup project")}</DialogHeader>
					<DialogBody>
						{tc("creating a backup...")}
					</DialogBody>
					<DialogFooter>
						<Button className="mr-1" onClick={cancel} disabled>{tc("cancel")}</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		default:
			let _: never = state;
	}

	return {startBackup, dialog}
}
