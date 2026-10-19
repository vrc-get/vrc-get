/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import globalInfo from "./global-info";

export function pathSeparators(): string[] {
	return globalInfo.osType === "WindowsNT" ? ["\\", "/"] : ["/"];
}

export function pathSeparator(): string {
	return pathSeparators()[0];
}

export function nameFromPath(path: string): string {
	if (globalInfo.osType === "WindowsNT") {
		const indexOfSlash = path.lastIndexOf("/");
		const indexOfBackSlash = path.lastIndexOf("\\");
		const indexOfSeparator = Math.max(indexOfSlash, indexOfBackSlash);
		if (indexOfSeparator === -1) return path;
		return path.substring(indexOfSeparator + 1);
	} else {
		return path.substring(path.lastIndexOf("/") + 1);
	}
}

export function directoryFromPath(path: string): string {
	if (globalInfo.osType === "WindowsNT") {
		const indexOfSlash = path.lastIndexOf("/");
		const indexOfBackSlash = path.lastIndexOf("\\");
		const indexOfSeparator = Math.max(indexOfSlash, indexOfBackSlash);
		if (indexOfSeparator === -1) return "";
		return path.substring(0, indexOfSeparator);
	} else {
		return path.substring(0, path.lastIndexOf("/"));
	}
}
