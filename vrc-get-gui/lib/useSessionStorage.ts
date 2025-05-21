import { useSyncExternalStore } from "react";

const subscribe = (callback: () => void) => {
	window.addEventListener("storage", callback);
	return () => {
		window.removeEventListener("storage", callback);
	};
};

export function useSessionStorage<TData>(options: {
	key: string;
	fallbackValue: TData;
	parse: (s: string) => TData;
}): TData {
	return useSyncExternalStore(subscribe, () => {
		const value = sessionStorage.getItem(options.key);
		if (value == null) return options.fallbackValue;
		return options.parse(value);
	});
}
