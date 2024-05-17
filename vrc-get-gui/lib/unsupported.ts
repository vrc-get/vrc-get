import i18next from "@/lib/i18n";
import {toastError} from "@/lib/toast";

export function unsupported(feature: string): () => void {
	return () => toastError(i18next.t("general:toast:not supported", {name: feature}))
}
