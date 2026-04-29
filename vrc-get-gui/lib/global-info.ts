import type { GlobalInfo as GlobalInfoBinding } from "./bindings.js";

type OsType = "Linux" | "Darwin" | "WindowsNT";
type Arch = "x86_64" | "aarch64";

type GlobalInfo = GlobalInfoBinding & {
	osType: OsType;
	arch: Arch;
};

const fallbackGlobalInfo: Readonly<GlobalInfo> = {
	language: "en",
	theme: "system",
	version: null,
	commitHash: null,
	osType: "WindowsNT",
	arch: "x86_64",
	osInfo: "unknown OS",
	webviewVersion: "unknown",
	appData: "",
	defaultUnityArguments: [],
	vpmHomeFolder: "",
	checkForUpdates: false,
	shouldInstallDeepLink: false,
};

const globalInfo: Readonly<GlobalInfo> = load();

function load(): GlobalInfo {
	if ("vrcGetGlobalInfo" in globalThis) {
		console.log("found vrcGetGlobalInfo!");
		// @ts-expect-error
		const info = globalThis.vrcGetGlobalInfo as GlobalInfo;
		onload(info);
		return info;
	}
	return fallbackGlobalInfo;
}

function onload(info: Readonly<GlobalInfo>) {
	document.documentElement.setAttribute("lang", info.language);
	document.documentElement.setAttribute("class", info.theme);
}

export default globalInfo;

export function useGlobalInfo(): Readonly<GlobalInfo> {
	return globalInfo;
}
