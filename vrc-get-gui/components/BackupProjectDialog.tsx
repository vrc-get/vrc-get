import type React from "react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import type { TauriCreateBackupProgress } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import type { DialogContext } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { toastNormal, toastSuccess } from "@/lib/toast";
import { useEffectEvent } from "@/lib/use-effect-event";

export function BackupProjectDialog({
	projectPath,
	dialog,
	header,
}: {
	projectPath: string;
	dialog: DialogContext<null | "cancelled">;
	header?: React.ReactNode;
}) {
	const [progress, setProgress] = useState<TauriCreateBackupProgress>({
		proceed: 0,
		total: 1,
		last_proceed: "",
	});

	const cancelRef = useRef<() => void>(undefined);

	const start = useEffectEvent(
		(projectPath: string, dialog: DialogContext<null | "cancelled">) => {
			const [cancel, promise] = callAsyncCommand(
				commands.projectCreateBackup,
				[projectPath],
				(progress) => {
					setProgress((prev) => {
						if (prev.proceed > progress.proceed) return prev;
						return progress;
					});
				},
			);

			cancelRef.current = cancel;

			promise
				.then((x) => {
					if (!header) {
						if (x === "cancelled") {
							toastNormal(tc("projects:toast:backup canceled"));
						} else {
							toastSuccess(tc("projects:toast:backup succeeded"));
						}
					}
					return x;
				})
				.then(dialog.close, dialog.error);
		},
	);

	useEffect(() => {
		start(projectPath, dialog);
	}, [projectPath, dialog]);

	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{header ?? tc("projects:dialog:backup header")}</DialogTitle>
			<DialogDescription>
				<p>{tc("projects:dialog:creating backup...")}</p>
				<p>
					{tc("projects:dialog:proceed k/n", {
						count: progress.proceed,
						total: progress.total,
					})}
				</p>
				<p className={"overflow-hidden w-full whitespace-pre"}>
					{progress.last_proceed || "Collecting files..."}
				</p>
				<Progress value={progress.proceed} max={progress.total} />
			</DialogDescription>
			<DialogFooter>
				<Button className="mr-1" onClick={() => cancelRef.current?.()}>
					{tc("general:button:cancel")}
				</Button>
			</DialogFooter>
		</div>
	);
}
