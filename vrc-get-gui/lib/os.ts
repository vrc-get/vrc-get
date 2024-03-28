import {OsType, type} from '@tauri-apps/api/os';

let osType: OsType | null = null;

(() => {
	type().then((osType_) => osType = osType_).catch(e => console.error(e));
})();

export function isWindows() {
	return osType === 'Windows_NT';
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
