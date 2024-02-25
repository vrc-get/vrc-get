import {TauriVersion} from "@/lib/bindings";

function comparePrereleaseSegment(a: string, b: string) {
	if (a === b) return 0;

	const aIsNum = !!a.match(/^\d+$/);
	const bIsNum = !!b.match(/^\d+$/);

	if (aIsNum) {
		if (bIsNum) {
			const aNum = parseInt(a, 10);
			const bNum = parseInt(b, 10);
			if (aNum < bNum) return -1;
			if (aNum > bNum) return 1;
			return 0;
		} else {
			return -1;
		}
	} else {
		if (!bIsNum) {
			// the js does < > in unicode code point order, which is the same as ASCII order
			if (a < b) return -1;
			if (a > b) return 1;
			return 0;
		} else {
			return 1;
		}
	}
}

export function compareVersion(a: TauriVersion, b: TauriVersion) {
	if (a.major < b.major) return -1;
	if (a.major > b.major) return 1;

	if (a.minor < b.minor) return -1;
	if (a.minor > b.minor) return 1;

	if (a.patch < b.patch) return -1;
	if (a.patch > b.patch) return 1;

	// fast path: exactly the same prerelease
	if (a.pre === b.pre) return 0;

	const aPrerelease = a.pre.split('.');
	const bPrerelease = b.pre.split('.');

	for (let i = 0; i < Math.min(aPrerelease.length, bPrerelease.length); i++) {
		const cmp = comparePrereleaseSegment(aPrerelease[i], bPrerelease[i]);
		if (cmp !== 0) return cmp;
	}

	if (aPrerelease.length < bPrerelease.length) return -1;
	if (aPrerelease.length > bPrerelease.length) return 1;

	return 0;
}

export function toVersionString(version: TauriVersion) : `${number}.${number}.${number}${`-${string}` | ''}${`+${string}` | ''}` {
	const versionString: `${number}.${number}.${number}` = `${version.major}.${version.minor}.${version.patch}`;
	const withPre: `${number}.${number}.${number}${`-${string}` | ''}` = version.pre ? `${versionString}-${version.pre}` : versionString;
	return version.build ? `${withPre}+${version.build}` : withPre;
}

export function compareUnityVersion(a: [number, number], b: [number, number]) {
	if (a[0] < b[0]) return -1;
	if (a[0] > b[0]) return 1;

	if (a[1] < b[1]) return -1;
	if (a[1] > b[1]) return 1;

	return 0;
}
