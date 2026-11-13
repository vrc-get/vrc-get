/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import type { EventCallback } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useEffectEvent } from "react";

export function useTauriListen<T>(event: string, handler: EventCallback<T>) {
	const handlerFn = useEffectEvent(handler);
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		let unlistened = false;

		listen<T>(event, handlerFn).then((unlistenFn) => {
			if (unlistened) {
				unlistenFn();
			} else {
				unlisten = unlistenFn;
			}
		});

		return () => {
			unlisten?.();
			unlistened = true;
		};
	}, [event]);
}
