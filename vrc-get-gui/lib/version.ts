import type { TauriVersion } from "@/lib/bindings";

function comparePrereleaseSegment(a: string, b: string) {
	if (a === b) return 0;

	const aIsNum = !!a.match(/^\d+$/);
	const bIsNum = !!b.match(/^\d+$/);

	if (aIsNum) {
		if (bIsNum) {
			const aNum = Number.parseInt(a, 10);
			const bNum = Number.parseInt(b, 10);
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

	// if either has no prerelease, it comes later
	if (a.pre === "") return 1;
	if (b.pre === "") return -1;

	const aPrerelease = a.pre.split(".");
	const bPrerelease = b.pre.split(".");

	for (let i = 0; i < Math.min(aPrerelease.length, bPrerelease.length); i++) {
		const cmp = comparePrereleaseSegment(aPrerelease[i], bPrerelease[i]);
		if (cmp !== 0) return cmp;
	}

	if (aPrerelease.length < bPrerelease.length) return -1;
	if (aPrerelease.length > bPrerelease.length) return 1;

	return 0;
}

export function toVersionString(
	version: TauriVersion,
): `${number}.${number}.${number}${`-${string}` | ""}${`+${string}` | ""}` {
	const versionString: `${number}.${number}.${number}` = `${version.major}.${version.minor}.${version.patch}`;
	const withPre: `${number}.${number}.${number}${`-${string}` | ""}` =
		version.pre ? `${versionString}-${version.pre}` : versionString;
	return version.build ? `${withPre}+${version.build}` : withPre;
}

export function compareUnityVersion(a: [number, number], b: [number, number]) {
	if (a[0] < b[0]) return -1;
	if (a[0] > b[0]) return 1;

	if (a[1] < b[1]) return -1;
	if (a[1] > b[1]) return 1;

	return 0;
}

export function compareUnityVersionString(a: string, b: string): 0 | 1 | -1 {
	if (a === b) return 0;

	const aParsed = parseUnityVersion(a);
	const bParsed = parseUnityVersion(b);

	if (!aParsed && !bParsed) {
		const cmp = a.localeCompare(b);
		return cmp < 0 ? -1 : cmp > 0 ? 1 : 0;
	} else if (!aParsed) {
		return 1;
	} else if (!bParsed) {
		return -1;
	}

	if (aParsed.major < bParsed.major) return -1;
	if (aParsed.major > bParsed.major) return 1;

	if (aParsed.minor < bParsed.minor) return -1;
	if (aParsed.minor > bParsed.minor) return 1;

	if (aParsed.patch < bParsed.patch) return -1;
	if (aParsed.patch > bParsed.patch) return 1;

	const channelCmp = compareUnityChannel(aParsed.channel, bParsed.channel);
	if (channelCmp !== 0) return channelCmp;

	if (aParsed.increment < bParsed.increment) return -1;
	if (aParsed.increment > bParsed.increment) return 1;

	return 0;
}

interface UnityVersion {
	major: number;
	minor: number;
	patch: number;
	channel: "a" | "b" | "f" | "c" | "p" | "x";
	increment: number;
}

export function parseUnityVersion(version: string): UnityVersion | null {
	let match = version.match(/^(\d+)\.(\d+)\.(\d+)([abfcpx])(\d+)$/);
	if (!match) {
		match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
		if (!match) {
			return null;
		}
	}
	return {
		major: Number.parseInt(match[1], 10),
		minor: Number.parseInt(match[2], 10),
		patch: Number.parseInt(match[3], 10),
		channel: (match[4] || "f") as UnityVersion["channel"],
		increment: Number.parseInt(match[5] || "1", 10),
	};
}

function compareUnityChannel(
	aIn: UnityVersion["channel"],
	bIn: UnityVersion["channel"],
) {
	const a = aIn === "c" ? "f" : aIn;
	const b = bIn === "c" ? "f" : bIn;

	if (a === b) return 0;

	if (a === "a") return -1;
	if (b === "a") return 1;

	if (a === "b") return -1;
	if (b === "b") return 1;

	if (a === "f") return -1;
	if (b === "f") return 1;

	if (a === "p") return 1;
	if (b === "p") return -1;

	if (a === "x") return -1;
	if (b === "x") return 1;

	return 0;
}
