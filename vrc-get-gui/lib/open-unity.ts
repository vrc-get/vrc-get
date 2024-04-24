import {projectOpenUnity} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import {toastError, toastNormal} from "@/lib/toast";

export async function openUnity(projectPath: string) {
	const result = await projectOpenUnity(projectPath);
	switch (result) {
		case "NoUnityVersionForTheProject":
			toastError(i18next.t("projects:toast:invalid project unity version"));
			break;
		case "NoMatchingUnityFound":
			toastError(i18next.t("projects:toast:match version unity not found"));
			break;
		case "Success":
			toastNormal(i18next.t("projects:toast:opening unity..."));
			break;
	}
}
