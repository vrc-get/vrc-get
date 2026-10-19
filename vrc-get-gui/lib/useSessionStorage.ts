/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import { useSyncExternalStore } from "react";

const subscribe = (callback: () => void) => {
	window.addEventListener("storage", callback);
	return () => {
		window.removeEventListener("storage", callback);
	};
};

export function useSessionStorage<TData>(options: {
	key: string;
	fallbackValue: TData;
	parse: (s: string) => TData;
}): TData {
	return useSyncExternalStore(subscribe, () => {
		const value = sessionStorage.getItem(options.key);
		if (value == null) return options.fallbackValue;
		return options.parse(value);
	});
}
