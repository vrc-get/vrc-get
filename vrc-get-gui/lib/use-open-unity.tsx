import {projectOpenUnity, TauriUnityVersions} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import {toastError, toastNormal} from "@/lib/toast";
import {useUnitySelectorDialog} from "@/lib/use-unity-selector-dialog";

export type OpenUnityFunction = (projectPath: string, unityVersion: string | null) => void;

export type Result = {
	dialog: React.ReactNode;
	openUnity: OpenUnityFunction;
}

export function useOpenUnity(unityVersions: TauriUnityVersions | undefined): Result {
	const unitySelector = useUnitySelectorDialog();

	const openUnity = async (projectPath: string, unityVersion: string | null) => {
		if (unityVersion == null) {
			toastError(i18next.t("projects:toast:invalid project unity version"));
			return;
		}
		if (unityVersions == null) {
			toastError(i18next.t("projects:toast:match version unity not found"));
			return;
		}

		const foundVersions = unityVersions.unity_paths.filter(([_p, v, _i]) => v === unityVersion);

		switch (foundVersions.length) {
			case 0:
				toastError(i18next.t("projects:toast:match version unity not found"));
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

	return {dialog: unitySelector.dialog, openUnity};
}
