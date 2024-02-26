import {projectOpenUnity} from "@/lib/bindings";
import {toast} from "react-toastify";

export async function openUnity(projectPath: string) {
	const result = await projectOpenUnity(projectPath);
	switch (result) {
		case "NoUnityVersionForTheProject":
			toast.error("We couldn't detect suitable Unity version for the project.");
			break;
		case "NoMatchingUnityFound":
			toast.error("No matching Unity version found. Please install or add a Unity version to the VCC.");
			break;
		case "Success":
			toast("Unity is opening...");
			break;
	}
}
