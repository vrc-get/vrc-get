// keep structure sync with uri_custom_scheme.rs
import { useEffect, useState } from "react";

type OsType = "Linux" | "Darwin" | "WindowsNT";
type Arch = "x86_64" | "aarch64";

interface GlobalInfo {
	language: string;
	theme: string;
	version: string | null;
	osType: OsType;
	arch: Arch;
	osInfo: string;
	localAppData: string; // empty string for non-windows
}

const fallbackGlobalInfo: Readonly<GlobalInfo> = {
	language: "en",
	theme: "system",
	version: null,
	osType: "WindowsNT",
	arch: "x86_64",
	osInfo: "unknown OS",
	localAppData: "",
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
	const [isClient, setIsClient] = useState(false);

	useEffect(() => {
		setIsClient(true);
	}, []);

	return isClient ? globalInfo : fallbackGlobalInfo;
}
