import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, CircleX, Ellipsis, Star } from "lucide-react";
import type React from "react";
import { Suspense, useId, useMemo, useState } from "react";
import { HeadingPageName } from "@/app/_main/packages/-tab-selector";
import Loading from "@/app/-loading";
import { FavoriteStarToggleButton } from "@/components/FavoriteStarButton";
import { HNavBar, VStack } from "@/components/layout";
import { Overlay } from "@/components/Overlay";
import {
	ReorderableList,
	useReorderableList,
} from "@/components/ReorderableList";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { TemplateSelect } from "@/components/TemplateSelect";
import {
	type AutoCompleteOption,
	Autocomplete,
} from "@/components/ui/autocomplete";
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
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	commands,
	type TauriAlcomTemplate,
	type TauriProjectTemplateInfo,
	type TauriVersion,
} from "@/lib/bindings";
import { dateToString, formatDateOffset } from "@/lib/dateToString";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc } from "@/lib/i18n";
import { processResult } from "@/lib/import-templates";
import { usePrevPathName } from "@/lib/prev-page";
import {
	type ProjectTemplateCategory,
	projectTemplateCategory,
	projectTemplateDisplayId,
	projectTemplateName,
} from "@/lib/project-template";
import { toastSuccess, toastThrownError } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { compareVersion } from "@/lib/version";

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
				className="shrink-0"
				leading={<HeadingPageName pageType={"/packages/templates"} />}
				trailing={
					<DropdownMenu>
						<div className={"flex divide-x"}>
							<CreateTemplateButton className={"rounded-r-none compact:h-10"} />
							<DropdownMenuTrigger
								asChild
								className={"rounded-l-none pl-2 pr-2 compact:h-10"}
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
				favoriteTemplates: information.data.favorite_templates,
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

	const templatesOrdered = useMemo(() => {
		const perCategoryFav: {
			[K in `${boolean}-${ProjectTemplateCategory}`]: TauriProjectTemplateInfo[];
		} = {
			"true-builtin": [],
			"false-builtin": [],
			"true-alcom": [],
			"false-alcom": [],
			"true-vcc": [],
			"false-vcc": [],
		};
		for (const template of information.data.templates) {
			const category = projectTemplateCategory(template.id);
			const favorite = information.data.favorite_templates.includes(
				template.id,
			);
			perCategoryFav[`${favorite}-${category}`].push(template);
		}
		return (["builtin", "alcom", "vcc"] as const).flatMap((category) => [
			...perCategoryFav[`true-${category}`],
			...perCategoryFav[`false-${category}`],
		]);
	}, [information.data.templates, information.data.favorite_templates]);

	return (
		<>
			<thead>
				<tr>
					<th
						className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground px-2.5 py-1.5`}
					>
						<Star className={"size-4"} />
					</th>
					{TABLE_HEAD.map((head, index) => (
						<th
							// biome-ignore lint/suspicious/noArrayIndexKey: static array
							key={index}
							className={
								"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground px-2.5 py-1.5"
							}
						>
							<small className="font-normal leading-none">{tc(head)}</small>
						</th>
					))}
				</tr>
			</thead>
			<tbody>
				{templatesOrdered.map((template) => (
					<TemplateRow
						key={template.id}
						template={template}
						remove={removeTemplate}
						edit={editTemplate}
						favorite={information.data.favorite_templates.includes(template.id)}
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
	favorite,
}: {
	template: TauriProjectTemplateInfo;
	remove?: (id: string) => void;
	edit?: (id: string) => void;
	favorite: boolean;
}) {
	const cellClass = "p-2.5 compact:py-1";
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

	const queryClient = useQueryClient();

	const setTemplateFavorite = useMutation({
		mutationFn: (params: { id: string; favorite: boolean }) =>
			commands.environmentSetTemplateFavorite(params.id, params.favorite),

		onMutate: async (params) => {
			await queryClient.cancelQueries(environmentProjectCreationInformation);

			const previousData = queryClient.getQueryData(
				environmentProjectCreationInformation.queryKey,
			);

			if (previousData !== undefined) {
				queryClient.setQueryData(
					environmentProjectCreationInformation.queryKey,
					{
						...previousData,
						favorite_templates: params.favorite
							? previousData.favorite_templates.includes(params.id)
								? previousData.favorite_templates
								: [...previousData.favorite_templates, params.id]
							: previousData.favorite_templates.filter((x) => x !== params.id),
					},
				);
			}

			return previousData;
		},

		onError: (error, _, context) => {
			console.error("Error favoriting project", error);
			toastThrownError(error);
			if (context) {
				queryClient.setQueryData(
					environmentProjectCreationInformation.queryKey,
					context,
				);
			}
		},
	});

	return (
		<tr className="even:bg-secondary/30 group">
			<td className={`${cellClass} w-3`}>
				<div className={"relative flex"}>
					<FavoriteStarToggleButton
						favorite={favorite}
						disabled={category === "vcc"}
						onToggle={() =>
							setTemplateFavorite.mutate({
								id: template.id,
								favorite: !favorite,
							})
						}
					/>
				</div>
			</td>
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
}: {
	dialog: DialogContext<boolean>;
	displayName: string;
}) {
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
						favoriteTemplates: information.data.favorite_templates,
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
const packageRangeRegex = new RegExp(
	regexp`^\s*(?:(?:>|<|>=|<=|=|\^|~)\s*)?v?${versionSegment}(?:\.${versionSegment}(?:\.${versionSegment}${prerelease}?${build}?)?)?\s*$`,
	"i",
);
// Currently, the unity version channel part and increment part is ignored and not allowed to include
const unityRangeRegex = new RegExp(
	regexp`^\s*(?:(?:>|<|>=|<=|=|\^|~)\s*)?v?${versionSegment}(?:\.${versionSegment}(?:\.${versionSegment})?)?\s*$`,
	"i",
);

function TemplateEditor({
	templates,
	template,
	favoriteTemplates,
	dialog,
}: {
	templates: TauriProjectTemplateInfo[];
	template: (TauriAlcomTemplate & { id: string }) | null;
	favoriteTemplates: string[];
	dialog: DialogContext<boolean>;
}) {
	const [baseTemplate, setBaseTemplate] = useState<string>(
		template?.base ?? "com.anatawa12.vrc-get.blank",
	);
	const [name, setName] = useState(template?.display_name ?? "");
	const [unityRange, setUnityRange] = useState(template?.unity_version ?? "");

	const allPackages = useQuery({
		queryKey: ["environmentPackages"],
		queryFn: () => commands.environmentPackages(),
	});

	const { packageCandidates, versionCandidatePerPackage } = useMemo(() => {
		type PackageInfo = {
			dataSourceVersion: TauriVersion;
			displayName: string | null;
			keywords: string[];
			versions: TauriVersion[];
		};
		const packages = new Map<string, PackageInfo>();
		for (const pkg of allPackages.data ?? []) {
			if (pkg.is_yanked) continue;
			let rowInfo = packages.get(pkg.name);
			if (
				rowInfo == null ||
				compareVersion(pkg.version, rowInfo.dataSourceVersion) > 0
			) {
				packages.set(
					pkg.name,
					(rowInfo = {
						dataSourceVersion: pkg.version,
						displayName: pkg.display_name,
						keywords: pkg.keywords,
						versions: rowInfo?.versions ?? [],
					}),
				);
			}
			rowInfo.versions.push(pkg.version);
		}
		return {
			packageCandidates: Array.from(packages.entries()).map(
				([id, pkg]) =>
					({
						value: id,
						label: (
							<AutocompletePackageLabel displayName={pkg.displayName} id={id} />
						),
						keywords: [pkg.displayName, ...pkg.keywords].filter(
							(x) => x != null,
						),
					}) satisfies AutoCompleteOption,
			),
			versionCandidatePerPackage: new Map(
				Array.from(packages.entries()).map(([id, pkg]) => {
					// we generate few candidates for version per package
					// - '*' for any version
					// - '>=latestStable' and '>=latestPrerelease'
					// - '^latestStable' and '^latestPrerelease'
					// - '1.x' '1.2.x' (or something like this) for stable release

					const latestStable = pkg.versions
						.filter((x) => x.pre === "")
						.sort(compareVersion)
						.at(-1);
					const latestPrerelease = pkg.versions.sort(compareVersion).at(-1);

					const candidates: AutoCompleteOption[] = [];

					function addCandidate(value: string, description: React.ReactNode) {
						candidates.push({
							value,
							label: (
								<AutocompleteVersionLabel
									value={value}
									description={description}
								/>
							),
						});
					}

					addCandidate("*", tc("templates:dialog:any version"));

					if (latestStable != null) {
						addCandidate(
							`${latestStable.major}.x`,
							`${latestStable.major}.0.0 ≤ v < ${latestStable.major + 1}.0.0`,
						);
						addCandidate(
							`${latestStable.major}.${latestStable.minor}.x`,
							`${latestStable.major}.${latestStable.minor}.0 ≤ v < ${latestStable.major}.${latestStable.minor + 1}.0`,
						);
						addCandidate(
							`${latestStable.major}.${latestStable.minor}.${latestStable.patch}`,
							`v = ${latestStable.major}.${latestStable.minor}.${latestStable.patch}`,
						);
						addCandidate(
							`>=${latestStable.major}.${latestStable.minor}.${latestStable.patch}`,
							`v ≥ ${latestStable.major}.${latestStable.minor}.${latestStable.patch}`,
						);
						addCandidate(
							`^${latestStable.major}.${latestStable.minor}.${latestStable.patch}`,
							`${latestStable.major}.${latestStable.minor}.${latestStable.patch} ≤ v < ${hatEndVersion(latestStable)}`,
						);
					}

					if (latestPrerelease != null && latestPrerelease !== latestStable) {
						addCandidate(
							`${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre}`,
							`v = ${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre}`,
						);
						addCandidate(
							`>=${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre}`,
							`v ≥ ${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre}`,
						);
						addCandidate(
							`^${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre}`,
							`${latestPrerelease.major}.${latestPrerelease.minor}.${latestPrerelease.patch}-${latestPrerelease.pre} ≤ v < ${hatEndVersion(latestPrerelease)}`,
						);
					}

					function hatEndVersion(version: TauriVersion): string {
						return version.major === 0 && version.minor === 0
							? `${version.major}.${version.minor}.${version.patch + 1}`
							: version.major === 0
								? `${version.major}.${version.minor + 1}.0`
								: `${version.major + 1}.0.0`;
					}

					return [id, candidates];
				}),
			),
		};
	}, [allPackages.data]);

	const unityCandidates = useMemo(() => {
		const templateInfo = templates.find((x) => x.id === baseTemplate);
		if (templateInfo == null) return [];
		// unityVersions is in order
		// currently, ignore the unity version channel part and increment part
		const unityVersions = templateInfo.unity_versions.map(
			(x) => x.split(/[^\d.]/, 2)[0],
		);
		const candidates: AutoCompleteOption[] = [];

		function addCandidate(value: string, description: React.ReactNode) {
			candidates.push({
				value,
				label: (
					<AutocompleteVersionLabel value={value} description={description} />
				),
			});
		}

		candidates.push(...unityVersions);

		addCandidate("*", tc("templates:dialog:any version"));

		// create something like 2022.x and 2022.3.x
		const addedRange = new Set<string>();
		for (const unityVersion of unityVersions) {
			const majorOnly = unityVersion.match(/^\d+/)?.[0];
			const minor = unityVersion.match(/^\d+\.\d+/)?.[0];
			if (majorOnly && !addedRange.has(majorOnly)) {
				addedRange.add(majorOnly);
				addCandidate(
					`${majorOnly}.x`,
					tc("templates:dialog:any unity specified version", {
						version: majorOnly,
					}),
				);
			}
			if (minor && !addedRange.has(minor)) {
				addedRange.add(minor);
				addCandidate(
					`${minor}.x`,
					tc("templates:dialog:any unity specified version", {
						version: minor,
					}),
				);
			}
		}
		return candidates;
	}, [templates, baseTemplate]);

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
		(p.name !== "" && p.range.match(packageRangeRegex)); // ready to create
	const readyToCreate =
		packagesListContext.value.every(validVersion) &&
		unityRange.match(unityRangeRegex) &&
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
										<TemplateSelect
											value={baseTemplate}
											onValueChange={setBaseTemplate}
											templates={templates}
											favoriteTemplates={favoriteTemplates}
											className={"grow"}
											excludeNoIdTemplates
										/>
									</td>
								</tr>
								<tr className={"contents"}>
									<th className={"content-center text-start whitespace-nowrap"}>
										{tc("templates:dialog:unity version")}:
									</th>
									<td className={"flex"}>
										<Autocomplete
											className={cn(
												"grow",
												unityRange.match(unityRangeRegex) ||
													"border-destructive ring-destructive text-destructive",
											)}
											value={unityRange}
											onChange={(value) => setUnityRange(value)}
											options={unityCandidates}
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
														<Autocomplete
															value={value.name}
															className={"grow"}
															options={packageCandidates}
															onChange={(value) =>
																packagesListContext.update(id, (old) => ({
																	...old,
																	name: value,
																}))
															}
														/>
													</div>
												</td>
												<td>
													<div className={"flex"}>
														<Autocomplete
															value={value.range}
															className={cn(
																"grow",
																validVersion(value) ||
																	"border-destructive ring-destructive text-destructive",
															)}
															options={
																versionCandidatePerPackage.get(value.name) ?? []
															}
															onChange={(value) =>
																packagesListContext.update(id, (old) => ({
																	...old,
																	range: value,
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

function AutocompletePackageLabel({
	displayName,
	id,
}: {
	displayName: string | null;
	id: string;
}) {
	if (displayName == null) return id;
	return (
		<div className={"flex flex-col"}>
			<div>{displayName}</div>
			<div className={"text-xs text-muted-foreground"}>{id}</div>
		</div>
	);
}

function AutocompleteVersionLabel({
	value,
	description,
}: {
	value: string;
	description: React.ReactNode;
}) {
	return (
		<div className={"flex flex-row justify-between w-full"}>
			<div>{value}</div>
			<div className={"text-xs text-muted-foreground"}>{description}</div>
		</div>
	);
}
