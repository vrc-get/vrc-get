let isWindowsCache: boolean | undefined

export async function loadOSApi(){
	const osApi = await import('@tauri-apps/api/os')
	return osApi
}

export function isWindows() {
	if (isWindowsCache === undefined) {
		isWindowsCache = navigator.userAgent.toLowerCase().includes("windows nt")
	}
	return isWindowsCache;
}

export function pathSeparators(): string[] {
	return isWindows() ? ['\\', '/'] : ['/'];
}

export function pathSeparator(): string {
	return pathSeparators()[0];
}

export function nameFromPath(path: string): string {
	if (isWindows()) {
		let indexOfSlash = path.lastIndexOf("/");
		let indexOfBackSlash = path.lastIndexOf("\\");
		let indexOfSeparator = Math.max(indexOfSlash, indexOfBackSlash);
		if (indexOfSeparator == -1) return path;
		return path.substring(indexOfSeparator + 1);
	} else {
		return path.substring(path.lastIndexOf("/") + 1);
	}
}
