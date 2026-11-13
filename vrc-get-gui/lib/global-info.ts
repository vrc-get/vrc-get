/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

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
	setupPages: [],
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
