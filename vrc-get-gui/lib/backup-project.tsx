import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import type { TauriCreateBackupProgress, TauriProject } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { tc, tt } from "@/lib/i18n";
import { toastNormal, toastSuccess, toastThrownError } from "@/lib/toast";
import { type ReactNode, useState } from "react";

// string if remove project by path
type Project =
	| TauriProject
	| {
			path: string;
			name: string;
	  };

type State =
	| {
			type: "idle";
	  }
	| {
			type: "backing-up";
			progress: TauriCreateBackupProgress;
			cancel: () => void;
	  };

type Result = {
	startBackup: (project: Project) => void;
	dialog: ReactNode;
};

export function useBackupProjectModal(): Result {
	const [state, setState] = useState<State>({ type: "idle" });

	const startBackup = async (project: Project) => {
		try {
			const [cancel, promise] = callAsyncCommand(
				commands.projectCreateBackup,
				[project.path],
				(progress) => {
					setState((state) => {
						if (state.type !== "backing-up") {
							return state;
						}
						if (state.progress.proceed >= progress.total) return state;
						return { ...state, progress };
					});
				},
			);
			setState({
				type: "backing-up",
				progress: {
					total: 100,
					proceed: 0,
					last_proceed: "",
				},
				cancel,
			});
			const channel = await promise;
			if (channel === "cancelled") {
				toastNormal(tt("projects:toast:backup canceled"));
			} else {
				toastSuccess(tt("projects:toast:backup succeeded"));
			}
			setState({ type: "idle" });
		} catch (e) {
			console.error("Error creating backup", e);
			setState({ type: "idle" });
			toastThrownError(e);
		}
	};

	let dialog: ReactNode = null;

	switch (state.type) {
		case "idle":
			break;
		case "backing-up":
			dialog = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:dialog:backup header")}</DialogTitle>
					<DialogDescription>
						<p>{tc("projects:dialog:creating backup...")}</p>
						<p>
							{tc("projects:dialog:proceed k/n", {
								count: state.progress.proceed,
								total: state.progress.total,
							})}
						</p>
						<p className={"overflow-hidden w-full whitespace-pre"}>
							{state.progress.last_proceed || "Collecting files..."}
						</p>
						<Progress
							value={state.progress.proceed}
							max={state.progress.total}
						/>
					</DialogDescription>
					<DialogFooter>
						<Button className="mr-1" onClick={state.cancel}>
							{tc("general:button:cancel")}
						</Button>
					</DialogFooter>
				</DialogOpen>
			);
			break;
		default:
			assertNever(state);
	}

	return { startBackup, dialog };
}
