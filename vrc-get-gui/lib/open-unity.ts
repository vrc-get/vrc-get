import {projectOpenUnity} from "@/lib/bindings";
import i18next from "@/lib/i18n";
import {toastError, toastNormal} from "@/lib/toast";

export async function openUnity(projectPath: string) {
	const result = await projectOpenUnity(projectPath);
	switch (result) {
		case "NoUnityVersionForTheProject":
			toastError(i18next.t("we couldn't detect suitable unity installations"));
			break;
		case "NoMatchingUnityFound":
			toastError(i18next.t("no matching unity version found. please install or add a unity version in the alcom settings"));
			break;
		case "Success":
			toastNormal(i18next.t("opening unity..."));
			break;
	}
}
