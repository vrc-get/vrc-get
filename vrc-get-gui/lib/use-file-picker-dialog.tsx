import { ReactNode, useCallback, useState } from "react";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
} from "@/components/ui/dialog";
import { tc } from "@/lib/i18n";

export function useFilePickerFunction<A extends unknown[], R>(
	f: (...args: A) => Promise<R>,
): [f: (...args: A) => Promise<R>, dialog: ReactNode] {
	let [isPicking, setIsPicking] = useState(false);
	let result = useCallback(
		async (...args: A) => {
			setIsPicking(true);
			try {
				return await f(...args);
			} finally {
				setIsPicking(false);
			}
		},
		[setIsPicking, f],
	);

	let dialog = (
		<Dialog open={isPicking}>
			<DialogContent>
				<DialogTitle>
					{tc("general:dialog:select file or directory header")}
				</DialogTitle>
				<DialogDescription>
					{tc("general:dialog:select file or directory")}
				</DialogDescription>
			</DialogContent>
		</Dialog>
	);

	return [result, dialog];
}
