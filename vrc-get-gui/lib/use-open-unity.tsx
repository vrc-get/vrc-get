import {projectOpenUnity, TauriUnityVersions} from "@/lib/bindings";
import i18next, {tc} from "@/lib/i18n";
import {toastError, toastNormal} from "@/lib/toast";
import {useUnitySelectorDialog} from "@/lib/use-unity-selector-dialog";
import {shellOpen} from "@/lib/shellOpen";
import {Button} from "@/components/ui/button";
import {Dialog, DialogContent, DialogDescription, DialogTitle} from "@/components/ui/dialog";
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

export function useOpenUnity(unityVersions: TauriUnityVersions | undefined): Result {
	const unitySelector = useUnitySelectorDialog();
	const [installStatus, setInstallStatus] = React.useState<StateInternal>({state: "normal"});

	const openUnity = async (projectPath: string, unityVersion: string | null, unityRevision?: string | null) => {
		if (unityVersion == null) {
			toastError(i18next.t("projects:toast:invalid project unity version"));
			return;
		}
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
			case 1:
				toastNormal(i18next.t("projects:toast:opening unity..."));
				await projectOpenUnity(projectPath, foundVersions[0][0]);
				return;
			default:
				const selected = await unitySelector.select(foundVersions);
				if (selected == null) return;
				toastNormal(i18next.t("projects:toast:opening unity..."));
				await projectOpenUnity(projectPath, selected);
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

	return <Dialog open>
    <DialogContent>
      <DialogTitle>
        {tc("projects:manage:dialog:unity not found")}
      </DialogTitle>
      <DialogDescription>
        <p>
          {tc("projects:manage:dialog:unity version of the project not found", {unity: expectedVersion})}
        </p>
      </DialogDescription>
      <div className={"ml-auto gap-2"}>
        <Button onClick={openUnityHub}>{tc("projects:manage:dialog:open unity hub")}</Button>
        <Button onClick={close} className="mr-1">{tc("general:button:close")}</Button>
      </div>
    </DialogContent>
	</Dialog>;
}

