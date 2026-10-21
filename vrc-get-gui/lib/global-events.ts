/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import { queryOptions } from "@tanstack/react-query";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import type {
	commands,
	TauriPackage,
	TauriUpdatedRealProjectInfo,
} from "@/lib/bindings";
import { queryClient } from "@/lib/query-client";

void tauriListen<TauriUpdatedRealProjectInfo>("projects-updated", (e) => {
	const options = queryOptions<
		Awaited<ReturnType<typeof commands.environmentProjects>>
	>({
		queryKey: ["environmentProjects"],
	});
	queryClient.setQueryData(options.queryKey, (old) => {
		if (old === undefined) return undefined;
		const index = old.projects.findIndex((p) => p.path === e.payload.path);
		if (index === -1) return old;
		const project = { ...old.projects[index] };
		project.is_valid = e.payload.is_valid;
		if (e.payload.is_valid) {
			project.unity = e.payload.unity;
			project.project_type = e.payload.project_type;
			project.unity_revision = e.payload.unity_revision;
		}
		const newList = [...old.projects];
		newList[index] = project;
		return { ...old, projects: newList };
	});
});

void tauriListen<TauriPackage[]>("package-update-background", (e) => {
	const options = queryOptions<
		Awaited<ReturnType<typeof commands.environmentPackages>>
	>({
		queryKey: ["environmentPackages"],
	});
	queryClient.setQueryData(options.queryKey, e.payload);
});

class EventSyncedVariable<T> {
	#value: T;
	#changeListeners: Set<() => void>;

	constructor(eventName: string, initialValue: T) {
		this.#value = initialValue;
		this.#changeListeners = new Set();
		this.useValue = this.useValue.bind(this);
		void tauriListen<T>(eventName, (e) => this.#setValue(e.payload));
	}

	get value(): T {
		return this.#value;
	}

	useValue() {
		return useSyncExternalStore<T>(this.#subscriber, this.#getValue);
	}

	#getValue = (): T => this.#value;

	#setValue = (value: T) => {
		this.#value = value;
		for (const listener of this.#changeListeners) listener();
	};

	#subscriber: (callcack: () => void) => () => void = (callcack) => {
		this.#changeListeners.add(callcack);
		return () => this.#changeListeners.delete(callcack);
	};
}

export const useProjectUpdateInProgress = new EventSyncedVariable(
	"projects-update-in-progress",
	false,
).useValue;

export const usePackageUpdateInProgress = new EventSyncedVariable(
	"package-update-in-progress",
	false,
).useValue;
