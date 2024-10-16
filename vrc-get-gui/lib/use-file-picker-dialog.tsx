import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
} from "@/components/ui/dialog";
import { tc } from "@/lib/i18n";
import { type ReactNode, useCallback, useState } from "react";

export function useFilePickerFunction<A extends unknown[], R>(
	f: (...args: A) => Promise<R>,
): [f: (...args: A) => Promise<R>, dialog: ReactNode] {
	const [isPicking, setIsPicking] = useState(false);
	const result = useCallback(
		async (...args: A) => {
			setIsPicking(true);
			try {
				return await f(...args);
			} finally {
				setIsPicking(false);
			}
		},
		[f],
	);

	const dialog = (
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
