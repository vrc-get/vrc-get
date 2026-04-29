import { useLocation } from "@tanstack/react-router";

let currentPath: string | null = null;
let prevPath: string | null = null;

export function usePrevPathName() {
	updateCurrentPath(useLocation().pathname);
	return prevPath ?? "";
}

export function updateCurrentPath(newPath: string) {
	if (currentPath === newPath) return;
	prevPath = currentPath;
	currentPath = newPath;
}
