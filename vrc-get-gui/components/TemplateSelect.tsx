import { useMemo } from "react";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { TauriProjectTemplateInfo } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import {
	type ProjectTemplateCategory,
	projectTemplateCategory,
	projectTemplateDisplayId,
	projectTemplateName,
} from "@/lib/project-template";

export function TemplateSelect({
	value,
	onValueChange,
	templates,
	favoriteTemplates,
	selectTriggerId,
	excludeNoIdTemplates = false,
	className,
}: {
	value: string;
	onValueChange: (templateId: string) => void;
	templates: TauriProjectTemplateInfo[];
	favoriteTemplates: string[];
	selectTriggerId?: string;
	excludeNoIdTemplates?: boolean;
	className?: string;
}) {
	const favoriteTemplatesSet = useMemo(
		() => new Set(favoriteTemplates),
		[favoriteTemplates],
	);

	type ProjectTemplateCategoryOrFav = ProjectTemplateCategory | "favorites";

	const templatesByCategory = useMemo(() => {
		const byCategory: {
			[k in ProjectTemplateCategory]: TauriProjectTemplateInfo[];
		} = {
			builtin: [],
			alcom: [],
			vcc: [],
		};
		const favorites: TauriProjectTemplateInfo[] = [];

		for (const template of templates) {
			if (excludeNoIdTemplates && projectTemplateDisplayId(template.id) == null)
				continue;
			if (favoriteTemplatesSet.has(template.id)) favorites.push(template);
			byCategory[projectTemplateCategory(template.id)].push(template);
		}

		return (
			[
				["favorites", favorites],
				["builtin", byCategory.builtin],
				["alcom", byCategory.alcom],
				["vcc", byCategory.vcc],
			] satisfies [ProjectTemplateCategoryOrFav, TauriProjectTemplateInfo[]][]
		).filter((x) => x[1].length > 0);
	}, [templates, favoriteTemplatesSet, excludeNoIdTemplates]);

	return (
		<Select
			value={`main-${value}`}
			onValueChange={(value) => onValueChange(value.replace(/^\w+-/, ""))}
		>
			<SelectTrigger id={selectTriggerId}>
				<SelectValue className={className} />
			</SelectTrigger>
			<SelectContent>
				{templatesByCategory.map(([category, templates], index) => (
					<SelectGroup key={category}>
						{index !== 0 && <SelectSeparator />}
						<SelectLabel>
							{tc(`projects:template-category:${category}`)}
						</SelectLabel>
						{templates.map((template) => {
							const disabled =
								!template.available || template.unity_versions.length === 0;
							const prefix =
								category === "favorites" ||
								!favoriteTemplatesSet.has(template.id)
									? "main"
									: "proxy";
							const contents = (
								<SelectItem
									value={`${prefix}-${template.id}`}
									disabled={disabled}
									key={template.id}
								>
									{projectTemplateName(template)}
								</SelectItem>
							);
							if (!template.available) {
								return (
									<Tooltip key={template.id}>
										<TooltipTrigger>{contents}</TooltipTrigger>
										<TooltipContent>
											{tc("projects:tooltip:template-unavailable")}
										</TooltipContent>
									</Tooltip>
								);
							} else if (template.unity_versions.length === 0) {
								return (
									<Tooltip key={template.id}>
										<TooltipTrigger>{contents}</TooltipTrigger>
										<TooltipContent>
											{tc("projects:tooltip:template-no-unity")}
										</TooltipContent>
									</Tooltip>
								);
							} else {
								return contents;
							}
						})}
					</SelectGroup>
				))}
			</SelectContent>
		</Select>
	);
}
