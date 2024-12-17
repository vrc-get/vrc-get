import globalInfo from "@/lib/global-info";
import { type DependencyList, useCallback, useEffect } from "react";

//declare interface DocumentEventMap {}
declare global {
	interface DocumentEventMap {
		"gui-animation": CustomEvent<boolean>;
	}
}

export function useDocumentEvent<EventName extends keyof DocumentEventMap>(
	eventName: EventName,
	listener: (event: DocumentEventMap[EventName]) => void,
	deps: DependencyList,
) {
	const listenerUse = useCallback(listener, deps);

	useEffect(() => {
		document.addEventListener(eventName, listenerUse);
		return () => {
			document.removeEventListener(eventName, listenerUse);
		};
	}, [eventName, listenerUse]);
}

export function isFindKey(
	e: Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey">,
) {
	if (e.key === "F3Key") return true;
	if ((globalInfo.osType === "Darwin" ? e.metaKey : e.ctrlKey) && e.key === "f")
		return true;
}
