import type {
	TauriPackage,
	TauriUpdatedRealProjectInfo,
	commands,
} from "@/lib/bindings";
import { queryClient } from "@/lib/query-client";
import { queryOptions } from "@tanstack/react-query";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";

void tauriListen<TauriUpdatedRealProjectInfo>("projects-updated", (e) => {
	const options = queryOptions<
		Awaited<ReturnType<typeof commands.environmentProjects>>
	>({
		queryKey: ["environmentProjects"],
	});
	queryClient.setQueryData(options.queryKey, (old) => {
		if (old === undefined) return undefined;
		const index = old.findIndex((p) => p.path === e.payload.path);
		if (index === -1) return old;
		const project = { ...old[index] };
		project.unity = e.payload.unity;
		project.project_type = e.payload.project_type;
		project.unity_revision = e.payload.unity_revision;
		const newList = [...old];
		newList[index] = project;
		return newList;
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
