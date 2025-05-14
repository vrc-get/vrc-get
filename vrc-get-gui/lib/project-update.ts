import type { TauriUpdatedRealProjectInfo, commands } from "@/lib/bindings";
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

let projectUpdateInProgress = false;
const projectUpdateInProgressListeners: (() => void)[] = [];

function setProjectUpdateInProgress(value: boolean) {
	projectUpdateInProgress = value;
	for (const l of projectUpdateInProgressListeners) {
		l();
	}
}

void tauriListen<TauriUpdatedRealProjectInfo>("projects-update-started", () =>
	setProjectUpdateInProgress(true),
);
void tauriListen<TauriUpdatedRealProjectInfo>("projects-update-finished", () =>
	setProjectUpdateInProgress(false),
);

function subscribeProjectUpdateInProgress(onChange: () => void): () => void {
	projectUpdateInProgressListeners.push(onChange);
	return () => {
		const index = projectUpdateInProgressListeners.indexOf(onChange);
		if (index !== -1) projectUpdateInProgressListeners.splice(index, 1);
	};
}

export function useProjectUpdateInProgress(): boolean {
	return useSyncExternalStore<boolean>(
		subscribeProjectUpdateInProgress,
		() => projectUpdateInProgress,
	);
}
