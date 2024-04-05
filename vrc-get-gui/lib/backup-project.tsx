import React, {ReactNode, useState} from "react";
import {Button, Dialog, DialogBody, DialogFooter, DialogHeader} from "@material-tailwind/react";
import {projectStartCreateBackup, TauriProject} from "@/lib/bindings";
import {toastNormal, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";
import {emit, listen, UnlistenFn} from "@tauri-apps/api/event";
import {nop} from "@/lib/nop";

// string if remove project by path
type Project = TauriProject | {
	path: string,
	name: string,
};

type State = {
	type: 'idle',
} | {
	type: 'backing-up',
	channel: string,
}

type Params = {}

type Result = {
	startBackup: (project: Project) => void,
	dialog: ReactNode,
}

export function useBackupProjectModal(_: Params = {}): Result {
	const [state, setState] = useState<State>({type: 'idle'});

	const startBackup = async (project: Project) => {
		try {
			const channel = await projectStartCreateBackup(project.path);
			setState({type: 'backing-up', channel});
			const canceled = await new Promise<boolean>((resolve, reject) => {
				let finishedListener: UnlistenFn | undefined;
				let failedListener: UnlistenFn | undefined;
				let canceledListener: UnlistenFn | undefined;

				const unlistenAll = <T, >(result: T) => {
					finishedListener?.();
					failedListener?.();
					canceledListener?.();
					return result;
				}

				listen(`${channel}:canceled`, () => unlistenAll(resolve(true))).then((listener) => canceledListener = listener);
				listen(`${channel}:finished`, () => unlistenAll(resolve(false))).then((listener) => finishedListener = listener);
				listen(`${channel}:failed`, (e) => unlistenAll(reject(e.payload))).then((listener) => failedListener = listener);
			})
			if (canceled) {
				toastNormal(tt("backup canceled"));
			} else {
				toastSuccess(tt("backup created successfully"));
			}
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
			const cancel = () => emit(`${state.channel}:cancel`);
			dialog = (
				<Dialog open handler={nop} className={'whitespace-normal'}>
					<DialogHeader>{tc("backup project")}</DialogHeader>
					<DialogBody>
						{tc("creating a backup...")}
					</DialogBody>
					<DialogFooter>
						<Button className="mr-1" onClick={cancel}>{tc("cancel")}</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		default:
			let _: never = state;
	}

	return {startBackup, dialog}
}
