import { useCallback, useRef } from "react";

/**
 * Something like useEffectEvent
 * @see https://ja.react.dev/learn/separating-events-from-effects#declaring-an-effect-event
 */
export function useEffectEvent<Args extends unknown[]>(
	listener: (...args: Args) => void,
): (...args: Args) => void {
	const event = useRef<(...args: Args) => void>();
	event.current = listener;

	return useCallback((...args: Args) => {
		if (event.current) {
			event.current(...args);
		}
	}, []);
}
