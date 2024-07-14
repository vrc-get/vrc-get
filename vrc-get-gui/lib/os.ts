import globalInfo from "./global-info";

export function pathSeparators(): string[] {
	return globalInfo.osType == "WindowsNT" ? ["\\", "/"] : ["/"];
}

export function pathSeparator(): string {
	return pathSeparators()[0];
}

export function nameFromPath(path: string): string {
	if (globalInfo.osInfo == "WindowsNT") {
		let indexOfSlash = path.lastIndexOf("/");
		let indexOfBackSlash = path.lastIndexOf("\\");
		let indexOfSeparator = Math.max(indexOfSlash, indexOfBackSlash);
		if (indexOfSeparator == -1) return path;
		return path.substring(indexOfSeparator + 1);
	} else {
		return path.substring(path.lastIndexOf("/") + 1);
	}
}
