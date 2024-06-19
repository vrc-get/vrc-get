import {
	environmentUnityVersions,
	projectGetUnityPath,
	projectOpenUnity,
	projectSetUnityPath,
	TauriUnityVersions
} from "@/lib/bindings";
import i18next, {tc} from "@/lib/i18n";
import {toastError, toastNormal} from "@/lib/toast";
import {useUnitySelectorDialog} from "@/lib/use-unity-selector-dialog";
import {shellOpen} from "@/lib/shellOpen";
import {Button} from "@/components/ui/button";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";
import React from "react";

export type OpenUnityFunction = (projectPath: string, unityVersion: string | null, unityRevision?: string | null) => void;

export type Result = {
	dialog: React.ReactNode;
	openUnity: OpenUnityFunction;
}

type StateInternal = {
	state: "normal";
} | {
	state: "suggest-unity-hub";
	unityVersion: string;
	unityHubLink: string;
}

export function useOpenUnity(): Result {
	const unitySelector = useUnitySelectorDialog();
	const [installStatus, setInstallStatus] = React.useState<StateInternal>({state: "normal"});

	const openUnity = async (projectPath: string, unityVersion: string | null, unityRevision?: string | null) => {
		if (unityVersion == null) {
			toastError(i18next.t("projects:toast:invalid project unity version"));
			return;
		}
		const [unityVersions, selectedPath] = await Promise.all([
			environmentUnityVersions(),
			projectGetUnityPath(projectPath),
		]);
		if (unityVersions == null) {
			toastError(i18next.t("projects:toast:match version unity not found", {unity: unityVersion}));
			return;
		}

		const foundVersions = unityVersions.unity_paths.filter(([_p, v, _i]) => v === unityVersion);

		switch (foundVersions.length) {
			case 0:
				if (unityRevision) {
					setInstallStatus({
						state: "suggest-unity-hub",
						unityVersion: unityVersion,
						unityHubLink: `unityhub://${unityVersion}/${unityRevision}`,
					});
				} else {
					toastError(i18next.t("projects:toast:match version unity not found", {unity: unityVersion}));
				}
				return;
			case 1: {
				if (selectedPath) {
					if (foundVersions[0][0] != selectedPath) {
						// if only unity is not
						void projectSetUnityPath(projectPath, null);
					}
				}
				const result = await projectOpenUnity(projectPath, foundVersions[0][0]);
				if (result)
					toastNormal(i18next.t("projects:toast:opening unity..."));
				else
					toastError(i18next.t("projects:toast:unity already running"));
			}
				return;
			default: {
				if (selectedPath) {
					const found = foundVersions.find(([p, _v, _i]) => p === selectedPath);
					if (found) {
						const result = await projectOpenUnity(projectPath, selectedPath);
						if (result)
							toastNormal(i18next.t("projects:toast:opening unity..."));
						else
							toastError(i18next.t("projects:toast:unity already running"));
						return;
					}
				}
				const selected = await unitySelector.select(foundVersions, true);
				if (selected == null) return;
				if (selected.keepUsingThisVersion) {
					void projectSetUnityPath(projectPath, selected.unityPath);
				}
				const result = await projectOpenUnity(projectPath, selected.unityPath);
				if (result)
					toastNormal(i18next.t("projects:toast:opening unity..."));
				else
					toastError("Unity already running");
			}
		}
	}

	const thisDialog = installStatus.state === "suggest-unity-hub" ? <UnityInstallWindow
		expectedVersion={installStatus.unityVersion}
		installWithUnityHubLink={installStatus.unityHubLink}
		close={() => setInstallStatus({state: "normal"})}
	/> : null;

	const dialog = <>
		{unitySelector.dialog}
		{thisDialog}
	</>

	return {dialog, openUnity};
}


function UnityInstallWindow(
	{
		expectedVersion,
		installWithUnityHubLink,
		close,
	}: {
		expectedVersion: string,
		installWithUnityHubLink: string,
		close: () => void
	}) {
	const openUnityHub = async () => {
		console.log("openUnityHub", installWithUnityHubLink)
		await shellOpen(installWithUnityHubLink);
	}

	return <DialogOpen>
		<DialogTitle>
			{tc("projects:manage:dialog:unity not found")}
		</DialogTitle>
		<DialogDescription>
			<p>
				{tc("projects:manage:dialog:unity version of the project not found", {unity: expectedVersion})}
			</p>
		</DialogDescription>
		<DialogFooter className={"gap-2"}>
			<Button onClick={openUnityHub}>{tc("projects:manage:dialog:open unity hub")}</Button>
			<Button onClick={close} className="mr-1">{tc("general:button:close")}</Button>
		</DialogFooter>
	</DialogOpen>;
}

