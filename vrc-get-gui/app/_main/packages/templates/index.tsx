import Loading from "@/app/-loading";
import { HeadingPageName } from "@/app/_main/packages/-tab-selector";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { HNavBar, VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { type TauriProjectTemplateInfo, commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { usePrevPathName } from "@/lib/prev-page";
import {
	projectTemplateCategory,
	projectTemplateName,
} from "@/lib/project-template";
import { useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CircleX } from "lucide-react";
import { Suspense, useId } from "react";

export const Route = createFileRoute("/_main/packages/templates/")({
	component: RouteComponent,
});

function RouteComponent() {
	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-left"
		: "";

	return (
		<VStack>
			<HNavBar
				className={"shrink-0"}
				leading={<HeadingPageName pageType={"/packages/templates"} />}
				trailing={<Button>{tc("templates:create template")}</Button>}
			/>
			<main
				className={`shrink overflow-hidden flex w-full h-full ${bodyAnimation}`}
			>
				<ScrollableCardTable className={"h-full w-full"}>
					<Suspense fallback={<Loading />}>
						<TemplatesTableBody />
					</Suspense>
				</ScrollableCardTable>
			</main>
		</VStack>
	);
}

function TemplatesTableBody() {
	const information = useSuspenseQuery({
		queryKey: ["environmentProjectCreationInformation"],
		queryFn: async () => await commands.environmentProjectCreationInformation(),
	});

	const TABLE_HEAD = [
		"general:name",
		"templates:category",
		"", // actions
	];

	return (
		<>
			<thead>
				<tr>
					{TABLE_HEAD.map((head, index) => (
						<th
							// biome-ignore lint/suspicious/noArrayIndexKey: static array
							key={index}
							className={
								"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
							}
						>
							<small className="font-normal leading-none">{tc(head)}</small>
						</th>
					))}
				</tr>
			</thead>
			<tbody>
				{information.data.templates.map((template) => (
					<TemplateRow key={template.id} template={template} />
				))}
			</tbody>
		</>
	);
}

function TemplateRow({
	template,
	remove,
}: {
	template: TauriProjectTemplateInfo;
	remove?: () => void;
}) {
	const cellClass = "p-2.5";
	const id = useId();

	const category = projectTemplateCategory(template.id);

	return (
		<tr className="even:bg-secondary/30">
			<td className={`${cellClass} w-full`}>
				<label htmlFor={id}>
					<p className="font-normal">{projectTemplateName(template)}</p>
				</label>
			</td>
			<td className={cellClass}>
				<Tooltip>
					<TooltipTrigger>
						<p className="font-normal">
							{tc(`templates:category:${category}`)}
						</p>
					</TooltipTrigger>
					<TooltipContent>
						{tc(`templates:tooltip:category:${category}`)}
					</TooltipContent>
				</Tooltip>
			</td>
			<td className={`${cellClass} w-min`}>
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							variant={"ghost"}
							size={"icon"}
							className={category !== "alcom" ? "opacity-50" : ""}
						>
							<CircleX className={"size-5 text-destructive"} />
						</Button>
					</TooltipTrigger>
					<TooltipContent>
						{category === "alcom"
							? tc("templates:tooltip:remove template")
							: category === "builtin"
								? tc("templates:tooltip:remove builtin template")
								: category === "vcc"
									? tc("templates:tooltip:remove vcc template")
									: ""}
					</TooltipContent>
				</Tooltip>
			</td>
		</tr>
	);
}
