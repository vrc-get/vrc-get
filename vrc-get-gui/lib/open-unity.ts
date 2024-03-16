import {projectOpenUnity} from "@/lib/bindings";
import {toast} from "react-toastify";
import i18next from "@/lib/i18n";

export async function openUnity(projectPath: string) {
	const result = await projectOpenUnity(projectPath);
	switch (result) {
		case "NoUnityVersionForTheProject":
			toast.error(i18next.t("we couldn't detect suitable unity installations"));
			break;
		case "NoMatchingUnityFound":
			toast.error(i18next.t("no matching unity version found. please install or add add a unity version in the vrc-get-gui settings"));
			break;
		case "Success":
			toast(i18next.t("opening unity..."));
			break;
	}
}
