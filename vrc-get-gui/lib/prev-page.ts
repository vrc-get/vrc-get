import { usePathname } from "next/navigation";

let currentPath: string | null = null;
let prevPath: string | null = null;

export function usePrevPathName() {
	updateCurrentPath(usePathname());
	return prevPath ?? "";
}

export function updateCurrentPath(newPath: string) {
	if (currentPath === newPath) return;
	prevPath = currentPath;
	currentPath = newPath;
}
