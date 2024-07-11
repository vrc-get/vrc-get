// keep structure sync with uri_custom_scheme.rs
interface GlobalInfo {
	language: string;
	theme: string;
}

const globalInfo: Readonly<GlobalInfo> = load();

function load(): GlobalInfo {
	if ('vrcGetGlobalInfo' in globalThis) {
		console.log("found vrcGetGlobalInfo!")
		// @ts-expect-error
		const info = globalThis.vrcGetGlobalInfo as GlobalInfo;
		onload(info);
		return info;
	}
	return {
		language: "en",
		theme: "system",
	}
}

function onload(info: Readonly<GlobalInfo>) {
	document.documentElement.setAttribute("lang", info.language);
	let theme = info.theme;
	if (theme === "system") {
		const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		theme = isDark ? "dark" : "light";
	}
	document.documentElement.setAttribute("class", theme);
}

export default globalInfo;
