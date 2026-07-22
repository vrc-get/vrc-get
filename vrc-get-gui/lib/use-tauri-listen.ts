import type { EventCallback } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useEffectEvent } from "react";

export function useTauriListen<T>(event: string, handler: EventCallback<T>) {
	const handlerFn = useEffectEvent(handler);
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		let unlistened = false;

		listen<T>(event, handlerFn).then((unlistenFn) => {
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
	}, [event]);
}
