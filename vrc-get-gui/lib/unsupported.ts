import {toast} from "react-toastify";
import i18next from "@/lib/i18n";

export function unsupported(feature: string): () => void {
	return () => toast.error(i18next.t("{{name}} is not supported yet", {name: feature}))
}
