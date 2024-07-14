import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { EventCallback } from "@tauri-apps/api/helpers/event";

export function useTauriListen<T>(event: string, handler: EventCallback<T>) {
	useEffect(() => {
		let unlisten: (() => void) | undefined = undefined;
		let unlistened = false;

		listen<T>(event, handler).then((unlistenFn) => {
			if (unlistened) {
				unlistenFn();
			} else {
				unlisten = unlistenFn;
			}
		});

		return () => {
			unlisten?.();
			unlistened = true;
		};
	}, [event, handler]);
}
