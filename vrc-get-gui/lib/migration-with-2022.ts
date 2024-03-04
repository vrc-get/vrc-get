import {listen} from "@tauri-apps/api/event";

export type TauriFinalizeMigrationWithUnity2022Event = {
	type: 'OutputLine',
	line: string,
} | {
	type: 'ExistsWithNonZero',
	status: string,
} | {
	type: 'FinishedSuccessfully',
} | {
	type: 'Failed',
}

type ProcessExitStatus = {
	type: 'ExistsWithNonZero',
	status: string,
} | {
	type: 'FinishedSuccessfully',
} | {
	type: 'Failed',
}

export function receiveLinesAndWaitForFinish(eventName: string, callback: (line: string) => void): Promise<ProcessExitStatus> {
	return new Promise((resolve, reject) => {
		let unlisten: (() => void) | null = null;
		listen<TauriFinalizeMigrationWithUnity2022Event>(eventName, event => {
			switch (event.payload.type) {
				case "OutputLine":
					callback(event.payload.line);
					break;

				case "FinishedSuccessfully":
					resolve({type: "FinishedSuccessfully"});
					unlisten?.();
					break;
				case "ExistsWithNonZero":
					resolve({type: "ExistsWithNonZero", status: event.payload.status});
					unlisten?.();
					break;
				case "Failed":
					resolve({type: "Failed"});
					unlisten?.();
					break;
				default:
					const _: never = event.payload;
			}
		}).then((f) => unlisten = f, reject);
	});
}
