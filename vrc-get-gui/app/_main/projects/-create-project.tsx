import { VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
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
import { assertNever } from "@/lib/assert-never";
import type {
	TauriProjectDirCheckResult,
	TauriProjectTemplateInfo,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { router } from "@/lib/main";
import { pathSeparator } from "@/lib/os";
import {
	type ProjectTemplateCategory,
	projectTemplateCategory,
	projectTemplateName,
} from "@/lib/project-template";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useDebounce } from "@uidotdev/usehooks";
import { RefreshCw } from "lucide-react";
import type React from "react";
import { useEffect } from "react";
import { useMemo } from "react";
import { useId } from "react";
import { useState } from "react";

export async function createProject() {
	using dialog = showDialog(<LoadingInitialInformation />);

	const information = await commands.environmentProjectCreationInformation();

	const result = await dialog.ask(EnteringInformation, {
		templates: information.templates,
		projectLocation: information.default_path,
	});

	if (result == null) return;

	dialog.replace(<CreatingProject />);

	await commands.environmentCreateProject(
		result.projectLocation,
		result.projectName,
		result.templateId,
		information.templates_version,
		result.unityVersion,
	);
	dialog.close();
	toastSuccess(tt("projects:toast:project created"));
	close?.();
	await queryClient.invalidateQueries({
		queryKey: ["environmentProjects"],
	});
	const projectPath = `${result.projectLocation}${pathSeparator()}${result.projectName}`;
	router.navigate({
		to: "/projects/manage",
		search: { projectPath },
	});
}

function DialogBase({
	children,
	close,
	createProject,
}: {
	children: React.ReactNode;
	close?: () => void;
	createProject?: () => void;
}) {
	return (
		<>
			<DialogTitle>{tc("projects:create new project")}</DialogTitle>
			<DialogDescription>{children}</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={close} disabled={!close}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={createProject} disabled={!createProject}>
					{tc("projects:button:create")}
				</Button>
			</DialogFooter>
		</>
	);
}

function LoadingInitialInformation() {
	return (
		<DialogBase>
			<RefreshCw className={"w-5 h-5 animate-spin"} />
		</DialogBase>
	);
}

interface ProjectCreationInformation {
	templateId: string;
	unityVersion: string;
	projectLocation: string;
	projectName: string;
}

function EnteringInformation({
	templates,
	projectLocation: projectLocationFirst,
	dialog,
}: {
	templates: TauriProjectTemplateInfo[];
	projectLocation: string;
	dialog: DialogContext<null | ProjectCreationInformation>;
}) {
	const [unityVersion, setUnityVersion] = useState<string>(
		(templates[0].unity_versions[0] || '').replace(/-([fpb]\d+)/gi, '$1')
	);
	const [templateId, setTemplateId] = useState<string>(templates[0].id);

	const templateById = useMemo(
		() => new Map(templates.map((t) => [t.id, t])),
		[templates],
	);

	const [projectNameRaw, setProjectName] = useState("New Project");
	const projectName = projectNameRaw.trim();
	const [projectLocation, setProjectLocation] = useState(projectLocationFirst);
	const projectNameCheckState = useProjectNameCheck(
		projectLocation,
		projectName,
	);

	const usePickProjectDefaultPath = useMutation({
		mutationFn: () => commands.environmentPickProjectDefaultPath(),
		onSuccess: (result) => {
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					setProjectLocation(result.new_path);
					break;
				default:
					assertNever(result);
			}
		},
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
	});

	const createProject = async () => {
		dialog.close({
			templateId,
			unityVersion,
			projectLocation,
			projectName,
		});
	};

	const templateInputId = useId();
	const unityInputId = useId();

	const templatesByCategory = useMemo(() => {
		const byCategory: {
			[k in ProjectTemplateCategory]: TauriProjectTemplateInfo[];
		} = {
			builtin: [],
			alcom: [],
			vcc: [],
		};

		for (const template of templates) {
			byCategory[projectTemplateCategory(template.id)].push(template);
		}

		return (
			[
				["builtin", byCategory.builtin],
				["alcom", byCategory.alcom],
				["vcc", byCategory.vcc],
			] satisfies [ProjectTemplateCategory, TauriProjectTemplateInfo[]][]
		).filter((x) => x[1].length > 0);
	}, [templates]);

	const selectedTemplateData = templateById.get(templateId);

	// Get the raw versions which might have a dash from custom templates
	const rawUnityVersions = selectedTemplateData?.unity_versions ?? [];

	// Normalize them to remove the dash before populating the dropdown
	const unityVersions = rawUnityVersions.map(version =>
		version.replace(/-([fpb]\d+)/gi, '$1')
	);

	const badProjectName = ["AlreadyExists", "InvalidNameForFolderName"].includes(
		projectNameCheckState,
	);

	const canCreateProject =
		projectNameCheckState !== "checking" && !badProjectName;

	useEffect(() => {
		// Log versions again when templateId changes
		const currentTemplateData = templateById.get(templateId);
		const currentRawVersions = currentTemplateData?.unity_versions ?? [];
		const currentNormalizedVersions = currentRawVersions.map(v => v.replace(/-([fpb]\d+)/gi, '$1'));
		if (currentNormalizedVersions.length > 0) {
			setUnityVersion(currentNormalizedVersions[0]);
		} else {
			setUnityVersion(''); // Or handle no available versions
		}
	}, [templateId, templateById]); // Rerun when template changes

	return (
		<DialogBase
			close={() => dialog.close(null)}
			createProject={canCreateProject ? createProject : undefined}
		>
			<VStack>
				<div className={"flex gap-1"}>
					<div className={"flex items-center whitespace-nowrap"}>
						<label htmlFor={templateInputId}>{tc("projects:template")}</label>
					</div>
					<Select
						value={templateId}
						onValueChange={(value) => setTemplateId(value)}
					>
						<SelectTrigger id={templateInputId}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{templatesByCategory.map(([category, templates], index) => (
								<SelectGroup key={category}>
									{index !== 0 && <SelectSeparator />}
									<SelectLabel>
										{tc(`projects:template-category:${category}`)}
									</SelectLabel>
									{templates.map((template) => {
										// Log each template's versions when rendering the list
										const itemRawVersions = template.unity_versions ?? [];
										const itemNormalizedVersions = itemRawVersions.map(v => v.replace(/-([fpb]\d+)/gi, '$1'));

										const disabled =
											!template.available ||
											itemNormalizedVersions.length === 0; // Check normalized length
										const contents = (
											<SelectItem
												value={template.id}
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
										} else if (itemNormalizedVersions.length === 0) {
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
				</div>
				<div className={"flex items-center gap-1 whitespace-nowrap"}>
					<label htmlFor={unityInputId}>
						{tc("projects:template:unity version")}
					</label>
					<Select
						value={unityVersion}
						onValueChange={(value) => setUnityVersion(value)}
						disabled={unityVersions.length === 1}
					>
						<SelectTrigger id={unityInputId}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{unityVersions.map((unityVersion) => (
								<SelectItem value={unityVersion} key={unityVersion}>
									<UnityVersion
										unityVersion={unityVersion}
										latestUnityVersion={
											unityVersions.length === 1 ? "" : unityVersions[0]
										}
									/>
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>
				<Input
					value={projectNameRaw}
					onChange={(e) => setProjectName(e.target.value)}
				/>
				<div className={"flex gap-1 items-center"}>
					<Input className="flex-auto" value={projectLocation} disabled />
					<Button
						className="flex-none px-4"
						onClick={() => usePickProjectDefaultPath.mutate()}
					>
						{tc("general:button:select")}
					</Button>
				</div>
				<small className={"whitespace-normal"}>
					{tc(
						"projects:hint:path of creating project",
						{ path: `${projectLocation}${pathSeparator()}${projectName}` },
						{
							components: {
								path: (
									<span
										className={
											"p-0.5 font-path whitespace-pre bg-secondary text-secondary-foreground"
										}
									/>
								),
							},
						},
					)}
				</small>
				<ProjectNameCheckResult projectNameCheckState={projectNameCheckState} />
			</VStack>
		</DialogBase>
	);
}

function useProjectNameCheck(
	projectLocation: string,
	projectName: string,
): "checking" | TauriProjectDirCheckResult {
	const projectNameDebounced = useDebounce(projectName, 500);

	const projectNameCheckStateTest = useQuery({
		queryKey: [
			"environmentCheckProjectName",
			projectLocation,
			projectNameDebounced,
		],
		queryFn: () =>
			commands.environmentCheckProjectName(
				projectLocation,
				projectNameDebounced,
			),
	});

	return projectNameDebounced !== projectName ||
		projectNameCheckStateTest.isFetching
		? "checking"
		: (projectNameCheckStateTest.data ?? "checking");
}

function ProjectNameCheckResult({
	projectNameCheckState,
}: {
	projectNameCheckState: "checking" | TauriProjectDirCheckResult;
}) {
	switch (projectNameCheckState) {
		case "Ok":
			return (
				<small className={"whitespace-normal text-success"}>
					{tc("projects:hint:create project ready")}
				</small>
			);
		case "InvalidNameForFolderName":
			return (
				<small className={"whitespace-normal text-destructive"}>
					{tc("projects:hint:invalid project name")}
				</small>
			);
		case "MayCompatibilityProblem":
			return (
				<small className={"whitespace-normal text-warning"}>
					{tc("projects:hint:warn symbol in project name")}
				</small>
			);
		case "WideChar":
			return (
				<small className={"whitespace-normal text-warning"}>
					{tc("projects:hint:warn multibyte char in project name")}
				</small>
			);
		case "AlreadyExists":
			return (
				<small className={"whitespace-normal text-destructive"}>
					{tc("projects:hint:project already exists")}
				</small>
			);
		case "checking":
			return (
				<small className={"whitespace-normal"}>
					<RefreshCw className={"w-5 h-5 animate-spin"} />
				</small>
			);
		default:
			assertNever(projectNameCheckState);
	}
}

function UnityVersion({
	unityVersion,
	latestUnityVersion,
}: {
	unityVersion: string;
	latestUnityVersion: string;
}) {
	if (unityVersion === latestUnityVersion) {
		return (
			<>
				{unityVersion}{" "}
				<span className={"text-success"}>{tc("projects:latest")}</span>
			</>
		);
	} else {
		return unityVersion;
	}
}

function CreatingProject() {
	return (
		<DialogBase>
			<RefreshCw className={"w-5 h-5 animate-spin"} />
			<p>{tc("projects:creating project...")}</p>
		</DialogBase>
	);
}
