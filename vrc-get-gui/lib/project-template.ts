import type { TauriProjectTemplateInfo } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import type React from "react";

const AVATARS_TEMPLATE_ID = "com.anatawa12.vrc-get.vrchat.avatars";
const WORLDS_TEMPLATE_ID = "com.anatawa12.vrc-get.vrchat.worlds";
const BLANK_TEMPLATE_ID = "com.anatawa12.vrc-get.blank";
const VCC_TEMPLATE_PREFIX = "com.anatawa12.vrc-get.vcc.";
const UNNAMED_TEMPLATE_PREFIX = "com.anatawa12.vrc-get.user.";

export function projectTemplateName(
	template: TauriProjectTemplateInfo,
): React.ReactNode {
	switch (template.id) {
		case AVATARS_TEMPLATE_ID:
			return tc("projects:template-name:avatars");
		case WORLDS_TEMPLATE_ID:
			return tc("projects:template-name:worlds");
		case BLANK_TEMPLATE_ID:
			return tc("projects:template-name:blank");
		default:
			return template.display_name;
	}
}

export type ProjectTemplateCategory = "builtin" | "alcom" | "vcc";

export const projectTemplateCategory = (
	id: string,
): ProjectTemplateCategory => {
	if (id.startsWith(VCC_TEMPLATE_PREFIX)) return "vcc";
	if (id.startsWith(UNNAMED_TEMPLATE_PREFIX)) return "alcom";
	if (id.startsWith("com.anatawa12.vrc-get.")) return "builtin";
	return "alcom";
};

export const projectTemplateDisplayId = (id: string): string | null => {
	if (id.startsWith(UNNAMED_TEMPLATE_PREFIX)) return null;
	if (id.startsWith(VCC_TEMPLATE_PREFIX)) return null;
	return id;
};
