import Loading from "@/app/-loading";
import { HeadingPageName } from "@/app/_main/packages/-tab-selector";
import { Overlay } from "@/components/Overlay";
import {
	ReorderableList,
	useReorderableList,
} from "@/components/ReorderableList";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { HNavBar, VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	type TauriAlcomTemplate,
	type TauriProjectTemplateInfo,
	commands,
} from "@/lib/bindings";
import { dateToString, formatDateOffset } from "@/lib/dateToString";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { processResult } from "@/lib/import-templates";
import { usePrevPathName } from "@/lib/prev-page";
import {
	projectTemplateCategory,
	projectTemplateDisplayId,
	projectTemplateName,
} from "@/lib/project-template";
import { toastSuccess, toastThrownError } from "@/lib/toast";
import { cn } from "@/lib/utils";
import {
	queryOptions,
	useQuery,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, CircleX, Ellipsis } from "lucide-react";
import type React from "react";
import { Suspense, useId, useState } from "react";

export const Route = createFileRoute("/_main/packages/templates/")({
	component: RouteComponent,
});

function RouteComponent() {
	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-left"
		: "";

	const importTemplates = async () => {
		try {
			await processResult(await commands.environmentImportTemplate());
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	return (
		<VStack>
			<HNavBar
				className={"shrink-0"}
				leading={<HeadingPageName pageType={"/packages/templates"} />}
				trailing={
					<DropdownMenu>
						<div className={"flex divide-x"}>
							<CreateTemplateButton className={"rounded-r-none"} />
							<DropdownMenuTrigger
								asChild
								className={"rounded-l-none pl-2 pr-2"}
							>
								<Button>
									<ChevronDown className={"w-4 h-4"} />
								</Button>
							</DropdownMenuTrigger>
						</div>
						<DropdownMenuContent>
							<DropdownMenuItem onClick={importTemplates}>
								{tc("templates:button:import template")}
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				}
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

const environmentProjectCreationInformation = queryOptions({
	queryKey: ["environmentProjectCreationInformation"],
	queryFn: async () => await commands.environmentProjectCreationInformation(),
});

function TemplatesTableBody() {
	const information = useSuspenseQuery(environmentProjectCreationInformation);

	const TABLE_HEAD = [
		"general:name",
		"templates:id",
		"general:last modified",
		"templates:category",
		"", // actions
	];

	const editTemplate = async (id: string) => {
		try {
			const alcomTemplate = await commands.environmentGetAlcomTemplate(id);
			await openSingleDialog(TemplateEditor, {
				templates: information.data.templates,
				template: { ...alcomTemplate, id },
			});
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const queryClient = useQueryClient();
	const removeTemplate = async (id: string) => {
		try {
			await commands.environmentRemoveTemplate(id);
			toastSuccess(tc("template:toast:removed"));
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		} finally {
			await queryClient.invalidateQueries(
				environmentProjectCreationInformation,
			);
		}
	};

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
					<TemplateRow
						key={template.id}
						template={template}
						remove={removeTemplate}
						edit={editTemplate}
					/>
				))}
			</tbody>
		</>
	);
}

function TemplateRow({
	template,
	remove,
	edit,
}: {
	template: TauriProjectTemplateInfo;
	remove?: (id: string) => void;
	edit?: (id: string) => void;
}) {
	const cellClass = "p-2.5";
	const id = useId();

	const category = projectTemplateCategory(template.id);
	const displayId = projectTemplateDisplayId(template.id);
	const lastModified = template.update_date;

	const deleteButton = async () => {
		if (
			await openSingleDialog(RemoveTemplateConfirmDialog, {
				displayName: template.display_name,
			})
		) {
			remove?.(template.id);
		}
	};

	return (
		<tr className="even:bg-secondary/30">
			<td className={`${cellClass} w-full`}>
				<label htmlFor={id}>
					<p className="font-normal">{projectTemplateName(template)}</p>
				</label>
			</td>
			<td className={cellClass}>
				{displayId ? (
					<p className="font-normal">{displayId}</p>
				) : (
					<p className="font-normal opacity-50">{tc("template:no id")}</p>
				)}
			</td>
			<td className={cellClass}>
				{lastModified ? (
					<Tooltip>
						<TooltipTrigger>{formatDateOffset(lastModified)}</TooltipTrigger>
						<TooltipContent>{dateToString(lastModified)}</TooltipContent>
					</Tooltip>
				) : (
					<p className="font-normal opacity-50">{tc("general:unknown date")}</p>
				)}
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
							onClick={category === "alcom" ? deleteButton : undefined}
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

				<TemplateDropdownMenu template={template} edit={edit} />
			</td>
		</tr>
	);
}

function RemoveTemplateConfirmDialog({
	dialog,
	displayName,
}: { dialog: DialogContext<boolean>; displayName: string }) {
	return (
		<>
			<DialogTitle>{tc("templates:dialog:remove template")}</DialogTitle>
			<DialogDescription>
				{tc("templates:dialog:confirm remove template", { displayName })}
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close(false)}>
					{tc("general:button:cancel")}
				</Button>
				<Button variant={"destructive"} onClick={() => dialog.close(true)}>
					{tc("general:button:delete")}
				</Button>
			</DialogFooter>
		</>
	);
}

function EllipsisButton(props: React.ComponentProps<typeof Button>) {
	return (
		<Button
			variant="ghost"
			size={"icon"}
			className={"hover:bg-primary/10 text-primary hover:text-primary"}
			{...props}
		>
			<Ellipsis className={"size-5"} />
		</Button>
	);
}

function TemplateDropdownMenu({
	template,
	edit,
}: {
	template: TauriProjectTemplateInfo;
	edit?: (id: string) => void;
}) {
	const category = projectTemplateCategory(template.id);

	// TODO: impleemnt edit template

	switch (category) {
		case "builtin":
			return <EllipsisButton disabled />;
		case "alcom": {
			const exportTemplate = async () => {
				try {
					await commands.environmentExportTemplate(template.id);
				} catch (e) {
					console.error(e);
					toastThrownError(e);
				}
			};
			return (
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<EllipsisButton />
					</DropdownMenuTrigger>
					<DropdownMenuContent>
						<DropdownMenuItem onClick={() => edit?.(template.id)}>
							{tc("templates:menuitem:edit template")}
						</DropdownMenuItem>
						{template.has_unitypackage ? (
							<Tooltip>
								<TooltipTrigger asChild>
									<DropdownMenuItem
										className={
											"opacity-50" /* emulate disabled. we cannot disable for tooltip */
										}
									>
										{tc("templates:menuitem:export template")}
									</DropdownMenuItem>
								</TooltipTrigger>
								<TooltipContent>
									{tc("templates:tooltip:export template with unitypackage")}
								</TooltipContent>
							</Tooltip>
						) : (
							<DropdownMenuItem onClick={exportTemplate}>
								{tc("templates:menuitem:export template")}
							</DropdownMenuItem>
						)}
					</DropdownMenuContent>
				</DropdownMenu>
			);
		}
		case "vcc": {
			const openTemplate = async () => {
				try {
					if (template.source_path == null)
						throw new Error("VCC Template path not found (bug(");
					await commands.utilOpen(template.source_path, "ErrorIfNotExists");
				} catch (e) {
					console.error(e);
					toastThrownError(e);
				}
			};
			return (
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<EllipsisButton />
					</DropdownMenuTrigger>
					<DropdownMenuContent>
						<DropdownMenuItem onClick={openTemplate}>
							{tc("templates:menuitem:open vcc template")}
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			);
		}
	}
}

function CreateTemplateButton({ className }: { className: string }) {
	const information = useQuery(environmentProjectCreationInformation);

	return (
		<Button
			disabled={information.isLoading}
			className={className}
			onClick={() => {
				if (information.data != null) {
					void openSingleDialog(TemplateEditor, {
						templates: information.data.templates,
						template: null,
					});
				}
			}}
		>
			{tc("templates:create template")}
		</Button>
	);
}

const regexp = String.raw;
const versionSegment = regexp`(?:\*|x|0|[1-9]\d*)`;
const prereleaseSegment = regexp`(?:0|[1-9]\d*|[0-9a-z-]*[a-z-][0-9a-z-]*)`;
const prerelease = regexp`(?:-?${prereleaseSegment}(?:\.${prereleaseSegment})*)`;
const buildSegment = regexp`(?:[0-9a-z-]+)`;
const build = regexp`(?:${buildSegment}(?:\.${buildSegment})*)`;
const rangeRegex = new RegExp(
	regexp`^\s*(?:(?:>|<|>=|<=|=|\^|~)\s*)?v?${versionSegment}(?:\.${versionSegment}(?:\.${versionSegment}${prerelease}?${build}?)?)?\s*$`,
	"i",
);

function TemplateEditor({
	templates,
	template,
	dialog,
}: {
	templates: TauriProjectTemplateInfo[];
	template: (TauriAlcomTemplate & { id: string }) | null;
	dialog: DialogContext<boolean>;
}) {
	const [baseTemplate, setBaseTemplate] = useState<string>(
		template?.base ?? "com.anatawa12.vrc-get.blank",
	);
	const [name, setName] = useState(template?.display_name ?? "");
	const [unityRange, setUnityRange] = useState(template?.unity_version ?? "");

	type Package = { name: string; range: string };
	const packagesListContext = useReorderableList<Package>({
		defaultValue: { name: "", range: "" },
		defaultArray:
			template == null
				? []
				: Object.entries(template.vpm_dependencies).map(([name, range]) => ({
						name,
						range,
					})),
		allowEmpty: false,
		reorderable: false,
	});

	const unityPackagesListContext = useReorderableList<string>({
		defaultValue: "",
		defaultArray: template?.unity_packages ?? [],
		allowEmpty: true,
		reorderable: false,
		addable: false,
	});

	const addUnityPackages = async () => {
		try {
			const packages = await commands.environmentPickUnityPackage();
			for (const pkg of packages) {
				unityPackagesListContext.add(pkg);
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const queryClient = useQueryClient();
	const saveTemplate = async () => {
		try {
			await commands.environmentSaveTemplate(
				template?.id ?? null,
				baseTemplate,
				name,
				unityRange,
				packagesListContext.value
					.filter((p) => !(p.name === "" && p.range === ""))
					.map(({ name, range }) => [name, range]),
				unityPackagesListContext.value,
			);
			await queryClient.invalidateQueries(
				environmentProjectCreationInformation,
			);
			dialog.close(true);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const validVersion = (p: Package) =>
		(p.name === "" && p.range === "") || // the empty (non-set) row
		(p.name !== "" && p.range.match(rangeRegex)); // ready to create
	const readyToCreate =
		packagesListContext.value.every(validVersion) &&
		unityRange.match(rangeRegex) &&
		name.length !== 0;

	return (
		<div className={"overflow-y-hidden flex flex-col"}>
			<DialogTitle>
				{template != null
					? tc("templates:dialog:edit template")
					: tc("templates:dialog:create template")}
			</DialogTitle>
			<DialogDescription asChild>
				<div className={"flex flex-col gap-4 shrink min-h-0"}>
					<section>
						<h3 className={"font-bold w-full text-center content-center"}>
							{tc("templates:dialog:general information")}
						</h3>
						<table
							className={"grid grid-cols-[min-content_1fr] gap-x-4 gap-y-1"}
						>
							<tbody className={"contents"}>
								<tr className={"contents"}>
									<th className={"content-center text-start whitespace-nowrap"}>
										{tc("general:name")}:
									</th>
									<td className={"flex"}>
										<Input
											className={cn(
												"grow",
												name.length === 0 &&
													"border-destructive ring-destructive text-destructive",
											)}
											value={name}
											onChange={(e) => setName(e.target.value)}
											placeholder={"Your New Template"}
										/>
									</td>
								</tr>
								<tr className={"contents"}>
									<th className={"content-center text-start whitespace-nowrap"}>
										{tc("templates:dialog:base template")}:
									</th>
									<td className={"flex"}>
										<Select
											value={baseTemplate}
											onValueChange={setBaseTemplate}
										>
											<SelectTrigger>
												<SelectValue className={"grow"} />
											</SelectTrigger>
											<SelectContent>
												{templates.map((template) => {
													const id = projectTemplateDisplayId(template.id);
													if (id == null) return null;
													return (
														<SelectItem key={id} value={id}>
															{projectTemplateName(template)}
														</SelectItem>
													);
												})}
											</SelectContent>
										</Select>
									</td>
								</tr>
								<tr className={"contents"}>
									<th className={"content-center text-start whitespace-nowrap"}>
										{tc("templates:dialog:unity version")}:
									</th>
									<td className={"flex"}>
										<Input
											className={cn(
												"grow",
												unityRange.match(rangeRegex) ||
													"border-destructive ring-destructive text-destructive",
											)}
											value={unityRange}
											onChange={(e) => setUnityRange(e.target.value)}
											placeholder={">=2022 * =2022.3.22"}
										/>
									</td>
								</tr>
							</tbody>
						</table>
					</section>
					<section className={"shrink overflow-hidden flex flex-col"}>
						<h3 className={"font-bold w-full text-center content-center"}>
							{tc("general:packages")}
						</h3>
						<div className={"w-full max-h-[30vh] overflow-y-auto shrink"}>
							<table className={"w-full"}>
								<thead>
									<tr>
										<th className={"sticky top-0 z-10 bg-background"}>
											{tc("general:name")}
										</th>
										<th className={"sticky top-0 z-10 bg-background"}>
											{tc("general:version")}
										</th>
										<th className={"sticky top-0 z-10 bg-background"} />
									</tr>
								</thead>
								<tbody>
									<ReorderableList
										context={packagesListContext}
										renderItem={(value, id) => (
											<>
												<td>
													<div className={"flex"}>
														<Input
															type={"text"}
															value={value.name}
															className={"grow"}
															onChange={(e) =>
																packagesListContext.update(id, (old) => ({
																	...old,
																	name: e.target.value,
																}))
															}
														/>
													</div>
												</td>
												<td>
													<div className={"flex"}>
														<Input
															type={"text"}
															value={value.range}
															className={cn(
																"grow",
																validVersion(value) ||
																	"border-destructive ring-destructive text-destructive",
															)}
															onChange={(e) =>
																packagesListContext.update(id, (old) => ({
																	...old,
																	range: e.target.value,
																}))
															}
														/>
													</div>
												</td>
											</>
										)}
									/>
								</tbody>
							</table>
						</div>
					</section>
					<section className={"shrink overflow-hidden flex flex-col"}>
						<Overlay>
							<h3 className={"font-bold w-full text-center content-center"}>
								{tc("templates:dialog:unitypackages")}
							</h3>
							<div className={"text-right mb-2"}>
								<Button onClick={addUnityPackages}>
									{tc("general:button:add")}
								</Button>
							</div>
						</Overlay>
						<div className={"w-full max-h-[30vh] overflow-y-auto shrink"}>
							<table className={"w-full"}>
								<tbody>
									<ReorderableList
										context={unityPackagesListContext}
										ifEmpty={() => (
											<td className={"text-center"}>
												{tc("templates:dialog:no unitypackages")}
											</td>
										)}
										renderItem={(value) => (
											<td>
												<div className={"flex"}>
													<Input
														type={"text"}
														value={value}
														className={"grow"}
														disabled
													/>
												</div>
											</td>
										)}
									/>
								</tbody>
							</table>
						</div>
					</section>
				</div>
			</DialogDescription>
			<DialogFooter className={"mt-2"}>
				<Button onClick={() => dialog.close(false)}>
					{tc("general:button:cancel")}
				</Button>
				<Button
					className={"ml-1"}
					disabled={!readyToCreate}
					onClick={saveTemplate}
				>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</div>
	);
}
