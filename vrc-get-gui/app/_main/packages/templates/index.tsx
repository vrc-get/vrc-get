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
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
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
import React, { Suspense, useId, useState } from "react";
import { VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { PackageMultiSelect } from "@/components/PackageMultiSelect";
import { combinePackagesAndProjectDetails } from "@/app/_main/projects/manage/-collect-package-row-info";
import globalInfo from "@/lib/global-info";

export const Route = createFileRoute("/_main/packages/templates/")({
	component: RouteComponent,
});

function RouteComponent() {
	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-left"
		: "";

	const queryClient = useQueryClient();
	const importTemplates = async () => {
		try {
			const count = await commands.environmentImportTemplate();
			await queryClient.invalidateQueries(
				environmentProjectCreationInformation,
			);
			if (count !== 0) {
				toastSuccess(tc("templates:toast:imported n templates", { count }));
			}
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
const environmentPackages = queryOptions({
	queryKey: ["environmentPackages"],
	queryFn: async () => await commands.environmentPackages(),
});
const environmentRepositoriesInfo = queryOptions({
	queryKey: ["environmentRepositoriesInfo"],
	queryFn: async () => await commands.environmentRepositoriesInfo(),
});
const environmentUnityVersions = queryOptions({
	queryKey: ["environmentUnityVersions"],
	queryFn: async () => await commands.environmentUnityVersions(),
});

function TemplatesTableBody() {
	const information = useSuspenseQuery(environmentProjectCreationInformation);

	const TABLE_HEAD = [
		"general:name",
		"templates:id",
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
		template?.base ?? "com.anatawa12.vrc-get.vrchat.avatars",
	);
	const [name, setName] = useState(template?.display_name ?? "New Template");
	const [unityRange, setUnityRange] = useState(template?.unity_version ?? VRCSDK_UNITY_VERSIONS[0]);
	const [nameTouched, setNameTouched] = useState(false);

	// Package IDs for each base template
	const AVATARS_PACKAGES = [
		"com.vrchat.avatars",
		"com.vrchat.base",
		"com.vrchat.core.vpm-resolver",
	];
	const WORLDS_PACKAGES = [
		"com.vrchat.worlds",
		"com.vrchat.base",
		"com.vrchat.core.vpm-resolver",
	];

	// State for selected packages (by package id)
	const [selectedPackages, setSelectedPackages] = useState<string[]>(
		template ? Object.keys(template.vpm_dependencies) : AVATARS_PACKAGES
	);

	// When baseTemplate changes, update selectedPackages accordingly
	React.useEffect(() => {
		if (baseTemplate === "com.anatawa12.vrc-get.vrchat.avatars") {
			setSelectedPackages((prev) => {
				// Remove worlds SDK if present, add avatars SDK
				let next = prev.filter(
					(id) => id !== "com.vrchat.worlds" && id !== "com.vrchat.avatars"
				);
				next = next.filter(
					(id) => !AVATARS_PACKAGES.includes(id)
				);
				return [...next, ...AVATARS_PACKAGES];
			});
		} else if (baseTemplate === "com.anatawa12.vrc-get.vrchat.worlds") {
			setSelectedPackages((prev) => {
				// Remove avatars SDK if present, add worlds SDK
				let next = prev.filter(
					(id) => id !== "com.vrchat.avatars" && id !== "com.vrchat.worlds"
				);
				next = next.filter(
					(id) => !WORLDS_PACKAGES.includes(id)
				);
				return [...next, ...WORLDS_PACKAGES];
			});
		} else if (baseTemplate === "com.anatawa12.vrc-get.blank") {
			setSelectedPackages((prev) => prev.filter(
				id =>
				id !== "com.vrchat.avatars" &&
				id !== "com.vrchat.worlds" &&
				id !== "com.vrchat.base" &&
				id !== "com.vrchat.core.vpm-resolver"
			));
		}
	}, [baseTemplate]);

	// Fetch available packages and repository info
	const information = useSuspenseQuery(environmentProjectCreationInformation);
	const packagesResult = useSuspenseQuery(environmentPackages);
	const repositoriesInfo = useSuspenseQuery(environmentRepositoriesInfo);
	const unityVersionsResult = useSuspenseQuery(environmentUnityVersions);

	const availablePackages = React.useMemo(
		() =>
			combinePackagesAndProjectDetails(
				Array.isArray(packagesResult.data) ? packagesResult.data : [],
				null,
				repositoriesInfo.data?.hidden_user_repositories ?? [],
				repositoriesInfo.data?.hide_local_user_packages ?? false,
				repositoriesInfo.data?.user_repositories ?? [],
				repositoriesInfo.data?.show_prerelease_packages ?? false,
			),
		[packagesResult.data, repositoriesInfo.data]
	);

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

	// Parse installed unity versions as version strings (e.g., '2022.3.22f1') from the unity_paths
	const installedUnityVersions = unityVersionsResult.data?.unity_paths?.map(([, version]) => version) || [];

	// Auto-select unity version if only one is installed, or select the first available if none is selected
	React.useEffect(() => {
		if (installedUnityVersions.length > 0 && (!unityRange || !installedUnityVersions.includes(unityRange))) {
			setUnityRange(installedUnityVersions[0] || '');
		}
	// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [installedUnityVersions.length]);

	const readyToCreate =
		selectedPackages.every(pkgId => availablePackages.find(p => p.id === pkgId)) &&
		unityRange.match(rangeRegex) &&
		name.length !== 0;

	const queryClient = useQueryClient();

	const saveTemplate = async () => {
		try {
			const vpmDependencies: Record<string, string> = {};
			for (const pkgId of selectedPackages) {
				vpmDependencies[pkgId] = "*";
			}
			// Normalize unityRange before saving (remove dash before f/p/b)
			let normalizedUnityRange = unityRange.replace(/-([fpb]\d+)$/i, '$1');
			await commands.environmentSaveTemplate(
				template?.id ?? null,
				baseTemplate,
				name,
				normalizedUnityRange,
				Object.entries(vpmDependencies),
				unityPackagesListContext.value as string[],
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

	return (
		<div className={"overflow-y-hidden flex flex-col max-w-[700px] p-8"} >
			<DialogTitle>
				{template != null
					? tc("templates:dialog:edit template")
					: tc("templates:dialog:create template")}
			</DialogTitle>
			<DialogDescription asChild>
				<div className={"flex flex-col gap-8 shrink min-h-0 overflow-y-auto"}>
					<section className="mb-8">
						<div className="flex justify-center items-center w-full mb-4">
							<h3 className="font-bold text-center">
								{tc("templates:dialog:general information")}
							</h3>
						</div>
						<table className={"grid grid-cols-[min-content_1fr] gap-x-4 gap-y-3"}>
							<tbody className={"contents"}>
								<tr className={"contents"}>
									<th className={"content-center text-start whitespace-nowrap"}>
										{tc("general:name")}:
									</th>
									<td className={"flex"}>
										<Input
											className={cn(
												"grow",
												name.length === 0 && nameTouched &&
												"border-destructive ring-destructive text-destructive",
											)}
											value={name}
											onChange={(e) => setName(e.target.value)}
											onBlur={() => setNameTouched(true)}
											placeholder={tc("templates:input:placeholder:new template name").toString()}
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
										<Select value={unityRange || ''} onValueChange={setUnityRange}>
											<SelectTrigger>
												<SelectValue className={"grow"} />
											</SelectTrigger>
											<SelectContent>
												{[...new Set(installedUnityVersions)].map((version) => (
													<SelectItem key={version} value={version}>
														{version}
													</SelectItem>
												))}
											</SelectContent>
										</Select>
									</td>
								</tr>
							</tbody>
						</table>
					</section>
					<section className="mb-8">
						<h3 className={"font-bold w-full text-center content-center mb-4"}>
							{tc("general:packages")}
						</h3>
						<div className="w-full">
							<PackageMultiSelect
								packages={availablePackages}
								selected={selectedPackages}
								onChange={setSelectedPackages}
								cellClassName="p-3 align-middle border-b border-secondary pl-6"
								headClassName="bg-secondary/50 border-b border-primary font-semibold"
								checkboxSeparator
							/>
						</div>
					</section>
					<section>
						<Overlay>
							<div className="flex items-center justify-between mb-2">
								<h3 className={"font-bold content-center"}>
									{tc("templates:dialog:unitypackages")}
								</h3>
								<Button onClick={addUnityPackages}>
									{tc("general:button:add")}
								</Button>
							</div>
						</Overlay>
						<label className="block mb-1 font-medium">
							{tc("templates:dialog:unitypackage path")}
						</label>
						<div className="w-full">
							<table className={"w-full align-middle"}>
								<tbody>
									<ReorderableList
										context={unityPackagesListContext}
										ifEmpty={() => (
											<td className={"text-center p-2 align-middle"}>
												{tc("templates:dialog:no unitypackages")}
											</td>
										)}
										renderItem={(value) => (
											<td className="p-2 align-middle">
												<div className={"flex items-center"}>
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
