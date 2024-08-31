import { BailoutToCSRError } from "next/dist/shared/lib/lazy-dynamic/bailout-to-csr";

type OsType = "Linux" | "Darwin" | "WindowsNT";
type Arch = "x86_64" | "aarch64";

interface GlobalInfo {
	language: string;
	theme: string;
	version: string | null;
	commitHash: string | null;
	osType: OsType;
	arch: Arch;
	osInfo: string;
	webviewVersion: string;
	localAppData: string; // empty string for non-windows
	defaultUnityArguments: string[];
	vpmHomeFolder: string;
}

const fallbackGlobalInfo: Readonly<GlobalInfo> = {
	language: "en",
	theme: "system",
	version: null,
	commitHash: null,
	osType: "WindowsNT",
	arch: "x86_64",
	osInfo: "unknown OS",
	webviewVersion: "unknown",
	localAppData: "",
	defaultUnityArguments: [],
	vpmHomeFolder: "",
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
	if (typeof window === "undefined") {
		throw new BailoutToCSRError("useGlobalInfo");
	}

	return globalInfo;
}
