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
import { tc, tt } from "@/lib/i18n";
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
	useSuspenseQueries,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, CircleX, Ellipsis, ChevronsUpDownIcon } from "lucide-react";
import React, { Suspense, useId, useState, useEffect, useRef, useMemo } from "react";
import { VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { PackageMultiSelect } from "@/app/_main/packages/templates/-PackageMultiSelect";
import { combinePackagesAndProjectDetails } from "@/app/_main/projects/manage/-collect-package-row-info";
import { Combobox } from "@/components/ui/combobox";
import { ScrollArea} from "@/components/ui/scroll-area";

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
					<p className="font-normal">{projectTemplateName(template as TauriProjectTemplateInfo)}</p>
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
						throw new Error(tc("general:error:vcc template path not found").toString());
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
	const hasInitializedDefaultUnityRange = useRef(false);
	const [unityRangeError, setUnityRangeError] = useState<string | null>(null);
	const [unityRangeTouched, setUnityRangeTouched] = useState(false);

	const normalizeSpecificVersion = (val: string): string => {
		if (!val) return ""; // Handle null, undefined, or empty string input
		let trimmedVal = val.trim();
		
		// Separate comparator if present
		let comparator = "";
		// Regex to match common semver comparators at the start of the string, 
		// followed by optional whitespace. It captures the comparator and the whitespace.
		const comparatorMatch = trimmedVal.match(/^(>=|<=|>|<|=|^|~)(\s*)/); 
		
		let versionPart = trimmedVal;

		if (comparatorMatch) {
			comparator = comparatorMatch[1]; // Just the comparator (e.g., ">=")
			// The version part is whatever comes after the full match (comparator + whitespace)
			versionPart = trimmedVal.substring(comparatorMatch[0].length).trimStart(); 
		}
		
		// Strip f, p, b suffixes (and potential preceding '-' for prereleases like -alpha.f1) 
		// from the version part.
		// This regex looks for -f[digits], f[digits], -p[digits], p[digits], etc. at the end of the string.
		versionPart = versionPart.replace(/-?[fpb][\d.]*$/i, ''); 
		
		// Re-attach comparator (if any) and the cleaned version string.
		if (comparator) {
			return `${comparator}${versionPart}`; // No extra space if we captured original spacing or want it tight
		}
		return versionPart;
	};

	const [baseTemplate, setBaseTemplate] = useState<string>(
		template?.base ?? "com.anatawa12.vrc-get.vrchat.avatars",
	);
	const [name, setName] = useState(template?.display_name ?? "New Template");

	const [unityRange, setUnityRange] = useState(() => {
		const initialValFromFile = template?.unity_version?.toString();
		if (initialValFromFile) {
			return normalizeSpecificVersion(initialValFromFile);
		}
		return "";
	});
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
	const BLANK_PACKAGES: string[] = []; // Explicitly define for clarity

	const BASE_TEMPLATE_PACKAGE_MAP: Record<string, string[]> = {
		"com.anatawa12.vrc-get.blank": BLANK_PACKAGES,
		"com.anatawa12.vrc-get.vrchat.avatars": AVATARS_PACKAGES,
		"com.anatawa12.vrc-get.vrchat.worlds": WORLDS_PACKAGES,
	};

	// State for selected packages (package id -> version string, e.g., "*" or specific version)
	const [userExplicitPackages, setUserExplicitPackages] = useState<Record<string, string>>(
		template ? template.vpm_dependencies : {}
	);

	// Fetch available packages and repository info
	const [
		{data: projectCreationInfo },
		{data: packagesData},
		{data: repositoriesInfoData},
		{data: unityVersionsData},
	] = useSuspenseQueries({
		queries: [
			environmentProjectCreationInformation,
			environmentPackages,
			environmentRepositoriesInfo,
			environmentUnityVersions,
		]
	});

	const availablePackages = React.useMemo(
		() =>
			combinePackagesAndProjectDetails(
				Array.isArray(packagesData) ? packagesData : [],
				null,
				repositoriesInfoData?.hidden_user_repositories ?? [],
				repositoriesInfoData?.hide_local_user_packages ?? false,
				repositoriesInfoData?.user_repositories ?? [],
				repositoriesInfoData?.show_prerelease_packages ?? false,
			),
		[packagesData, repositoriesInfoData]
	);

	const implicitPackagesForCurrentBase = useMemo(
		() => BASE_TEMPLATE_PACKAGE_MAP[baseTemplate] || [],
		[baseTemplate],
	);

	// Determine SDK packages that conflict with the current base template choice
	const conflictingSdkPackagesToHide = useMemo(() => {
		if (baseTemplate === "com.anatawa12.vrc-get.vrchat.avatars") {
			return ["com.vrchat.worlds"];
		} else if (baseTemplate === "com.anatawa12.vrc-get.vrchat.worlds") {
			return ["com.vrchat.avatars"];
		}
		return [];
	}, [baseTemplate]);

	// Combine all packages that should not appear in the multi-select checklist
	const allPackagesToHideInChecklist = useMemo(() => {
		return new Set([...implicitPackagesForCurrentBase, ...conflictingSdkPackagesToHide]);
	}, [implicitPackagesForCurrentBase, conflictingSdkPackagesToHide]);

	const packagesForMultiSelect = React.useMemo(
		() =>
			availablePackages.filter(
				(pkg) => !allPackagesToHideInChecklist.has(pkg.id),
			),
		[availablePackages, allPackagesToHideInChecklist],
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
	// and strip fX, pX, bX suffixes
	const installedUnityVersions = React.useMemo(() => {
		const paths = unityVersionsData?.unity_paths;
		if (!paths) return [];
		const versions = paths.map(([, version]) => {
			if (typeof version !== 'string') {
				return "INVALID_VERSION_TYPE"; 
			}
			return version.replace(/[fpb]\d*$/i, '');
		});
		const uniqueVersions = versions.filter((v, i, a) => a.indexOf(v) === i); 
		return uniqueVersions;
	}, [unityVersionsData]);

	// Determine options for the Combobox based on selected base template or installed versions
	const comboboxOptions = React.useMemo(() => {
		const selectedBaseTemplateInfo = templates.find(t => projectTemplateDisplayId(t.id) === baseTemplate);
		let versionsToConsider: string[] = installedUnityVersions;
		let supportedVersions: string[] = [];

		if (baseTemplate === "com.anatawa12.vrc-get.blank") {
			supportedVersions = VRCSDK_UNITY_VERSIONS.map(v => normalizeSpecificVersion(v));
		} else if (selectedBaseTemplateInfo?.unity_versions?.length) {
			supportedVersions = selectedBaseTemplateInfo.unity_versions.map(v => normalizeSpecificVersion(String(v)));
		} else {
			supportedVersions = VRCSDK_UNITY_VERSIONS.map(v => normalizeSpecificVersion(v));
		}
        
		const uniqueNormalizedVersions = versionsToConsider
			.map(v => normalizeSpecificVersion(String(v)))
			.filter((v, i, a) => a.indexOf(v) === i && v && v !== "INVALID_VERSION_TYPE");

		// Create options for all installed Unity versions
		const installedOpts = uniqueNormalizedVersions.map(v => ({
			label: v,
			value: v,
			isInstalled: true,
			isSupported: supportedVersions.includes(v)
		}));

		// Add supported versions that aren't installed
		const supportedNotInstalled = supportedVersions
			.filter(v => !uniqueNormalizedVersions.includes(v))
			.map(v => ({
				label: v,
				value: v,
				isInstalled: false,
				isSupported: true
			}));

		return [...installedOpts, ...supportedNotInstalled];
	}, [baseTemplate, templates, installedUnityVersions, normalizeSpecificVersion]);

	const [isOpen, setIsOpen] = useState(false);

	function handleUnityVersionSelect(version: string) {
		setUnityRange(normalizeSpecificVersion(version));
		setUnityRangeTouched(true);
		setIsOpen(false);
	}

	useEffect(() => {
		if (template) { 
			const templateVersion = template.unity_version?.toString();
			const normalizedTemplateVersion = templateVersion ? normalizeSpecificVersion(templateVersion) : "";
			if (normalizedTemplateVersion && unityRange !== normalizedTemplateVersion) {
				setUnityRange(normalizedTemplateVersion);
			} else if (!templateVersion && unityRange !== "") { 
				 setUnityRange("");
			}
			if (normalizedTemplateVersion) {
				setUnityRangeTouched(true);
			}
		} else { 
			setUnityRange(""); 
			hasInitializedDefaultUnityRange.current = false;
		}
	}, [template]);

	useEffect(() => {
		const normalized = normalizeSpecificVersion(unityRange).trim();
		if (unityRangeTouched && normalized.length > 0 && !rangeRegex.test(normalized)) {
			setUnityRangeError(tt("templates:dialog:invalid unity version format"));
		} else {
			setUnityRangeError(null);
		}
	}, [unityRange, unityRangeTouched, normalizeSpecificVersion]);

	const normalizedAndTrimmedUnityRange = normalizeSpecificVersion(unityRange).trim();
	const isUnityRangeActuallyValid = normalizedAndTrimmedUnityRange.length > 0 && rangeRegex.test(normalizedAndTrimmedUnityRange);

	const readyToCreate =
		Object.keys(userExplicitPackages).every(pkgId => availablePackages.find(p => p.id === pkgId)) &&
		name.trim().length !== 0 &&
		isUnityRangeActuallyValid;

	const queryClient = useQueryClient();

	const saveTemplate = async () => {
		try {
			const finalVpmDependencies: Record<string, string> = { ...userExplicitPackages };

			const implicitPkgIds = BASE_TEMPLATE_PACKAGE_MAP[baseTemplate] || [];
			for (const pkgId of implicitPkgIds) {
				if (!(pkgId in finalVpmDependencies)) {
					finalVpmDependencies[pkgId] = "*"; 
				}
			}

			const finalUnityRange = normalizeSpecificVersion(unityRange);

			await commands.environmentSaveTemplate(
				template?.id ?? null,
				baseTemplate,
				name,
				finalUnityRange,
				Object.entries(finalVpmDependencies),
				unityPackagesListContext.value as string[]
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
		<div className={"flex flex-col max-w-4xl p-0 h-[85vh] overflow-hidden"}>
			<header className="p-8 shrink-0">
				<DialogTitle>
					{template != null
						? tc("templates:dialog:edit template")
						: tc("templates:dialog:create template")}
				</DialogTitle>
			</header>

			<DialogDescription
				asChild
				className="flex-1 min-h-0 overflow-hidden"
			>
				<ScrollArea
					className="h-full px-8 vrc-get-scrollable-card"
					scrollBarClassName="bg-background py-2.5 vrc-get-scrollable-card-vertical-bar"
				>
					<div className={"flex flex-col gap-8 pr-4 pb-32"}>
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
												placeholder={tt("templates:input:placeholder:new template name")}
											/>
										</td>
									</tr>
									<tr className={"contents"}>
										<th className={"content-start text-start whitespace-nowrap pt-3"}>
											{tc("templates:dialog:base template")}:
										</th>
										<td className={"flex items-start"}>
											<div className={"flex flex-row gap-2"}>
												<div className="flex flex-col">
													<Select
														value={baseTemplate}
														onValueChange={(value) => setBaseTemplate(value)}
														name={"base-template-select"}
													>
														<SelectTrigger>
															<SelectValue />
														</SelectTrigger>
														<SelectContent>
															{projectCreationInfo?.templates
																?.filter((t) => projectTemplateCategory(t.id) === "builtin")
																.map((template) => (
																	<SelectItem
																		key={template.id}
																		value={template.id}
																		disabled={template.id.includes(".vcc.")}
																	>
																		{projectTemplateName(template as TauriProjectTemplateInfo)}
																	</SelectItem>
																))}
														</SelectContent>
													</Select>
													{BASE_TEMPLATE_PACKAGE_MAP[baseTemplate] && BASE_TEMPLATE_PACKAGE_MAP[baseTemplate].length > 0 && (
														<p className="text-sm text-muted-foreground mt-1">
															{tc("templates:dialog:implicitly includes")}: {BASE_TEMPLATE_PACKAGE_MAP[baseTemplate].join(", ")}
														</p>
													)}
												</div>
											</div>
										</td>
									</tr>
									<tr className={"contents"}>
										<th className={"content-center text-start whitespace-nowrap"}>
											{tc("templates:dialog:unity version")}:
										</th>
										<td className={"flex flex-col"}>
											<Combobox
												options={comboboxOptions}
												value={unityRange}
												onValueChange={handleUnityVersionSelect}
												placeholder=">=2022 * =2022.3.22"
												emptyStateMessage={tt("templates:dialog:no unity version found")}
												className={cn(
													"w-full",
													unityRangeError && "border-2 border-destructive"
												)}
											/>
											{unityRangeError && (
												<>
													<p className="text-destructive text-sm mt-1">{unityRangeError}</p>
													<p className="text-destructive text-sm mt-1">{tt("templates:dialog:invalid unity version format hint")}</p>
												</>
											)}
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
									packages={packagesForMultiSelect}
									selected={userExplicitPackages}
									onChange={setUserExplicitPackages}
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
							<table className={"w-full align-middle"}>
								<ReorderableList
									context={unityPackagesListContext}
									ifEmpty={() => (
										<tr>
											<td
												colSpan={2}
												className={"text-center p-2 align-middle"}
											>
												{tc("templates:dialog:no unitypackages")}
											</td>
										</tr>
									)}
									renderItem={(value) => (
										<tr key={value}>
											<td className="p-2 align-middle w-full">
												<div className={"flex items-center"}>
													<Input
														type={"text"}
														value={value}
														className={"grow"}
														disabled
													/>
												</div>
											</td>
											<td className="w-1">
												{/* Placeholder for delete/drag handle buttons if needed in the future */}
											</td>
										</tr>
									)}
								/>
							</table>
						</section>
					</div>
				</ScrollArea>
			</DialogDescription>

			<footer className="sticky bottom-0 shrink-0 p-8 bg-background z-10 border-t border-border">
				<DialogFooter className="flex justify-end gap-2">
					<Button variant="default" onClick={() => dialog.close(false)}>
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
			</footer>
		</div>
	);
}
