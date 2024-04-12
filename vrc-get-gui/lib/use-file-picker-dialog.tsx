import {ReactNode, useCallback, useState} from "react";
import {Dialog, DialogBody, DialogHeader} from "@material-tailwind/react";
import {nop} from "@/lib/nop";
import {tc} from "@/lib/i18n";

export function useFilePickerFunction<A extends unknown[], R>(
	f: (...args: A) => Promise<R>,
): [f: (...args: A) => Promise<R>, dialog: ReactNode] {
	let [isPicking, setIsPicking] = useState(false);
	let result = useCallback(async (...args: A) => {
		setIsPicking(true);
		try {
			return await f(...args);
		} finally {
			setIsPicking(false);
		}
	}, [setIsPicking, f]);

	let dialog = <Dialog open={isPicking} handler={nop}>
		<DialogHeader>{tc("selecting file or folder")}</DialogHeader>
		<DialogBody>{tc("please select a file or folder")}</DialogBody>
	</Dialog>;

	return [result, dialog];
}
