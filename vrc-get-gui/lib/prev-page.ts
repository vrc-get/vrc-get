/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import { useLocation } from "@tanstack/react-router";

let currentPath: string | null = null;
let prevPath: string | null = null;

export function usePrevPathName() {
	updateCurrentPath(useLocation().pathname);
	return prevPath ?? "";
}

export function updateCurrentPath(newPath: string) {
	if (currentPath === newPath) return;
	prevPath = currentPath;
	currentPath = newPath;
}
