import { useMutation } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import type React from "react";
import { useEffect, useId, useMemo, useState } from "react";
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
import type { TauriProjectTemplateInfo } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { router } from "@/lib/main";
import { pathSeparator } from "@/lib/os";
import {
	ProjectNameCheckResult,
	useProjectNameCheck,
} from "@/lib/project-name-check";
import {
	type ProjectTemplateCategory,
	projectTemplateCategory,
	projectTemplateName,
} from "@/lib/project-template";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";

export async function createProject() {
	const information = await commands.environmentProjectCreationInformation();

	using dialog = showDialog();
	const result = await dialog.ask(EnteringInformation, {
		templates: information.templates,
		projectLocation: information.default_path,
		recentProjectLocations: information.recent_project_locations,
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

interface ProjectCreationInformation {
	templateId: string;
	unityVersion: string;
	projectLocation: string;
	projectName: string;
}

function EnteringInformation({
	templates,
	projectLocation: projectLocationFirst,
	recentProjectLocations: recentProjectLocationsReversed,
	dialog,
}: {
	templates: TauriProjectTemplateInfo[];
	projectLocation: string;
	recentProjectLocations: string[];
	dialog: DialogContext<null | ProjectCreationInformation>;
}) {
	const [unityVersion, setUnityVersion] = useState<string>(
		templates[0].unity_versions[0],
	);
	const [templateId, setTemplateId] = useState<string>(templates[0].id);

	const templateById = useMemo(
		() => new Map(templates.map((t) => [t.id, t])),
		[templates],
	);

	const [projectNameRaw, setProjectName] = useState("New Project");
	const projectName = projectNameRaw.trim();
	const [projectLocation, setProjectLocation] = useState(projectLocationFirst);
	const [lastPickedLocation, setLastPickedLocation] =
		useState(projectLocationFirst);
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
					setLastPickedLocation(result.new_path);
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

	const unityVersions = templateById.get(templateId)?.unity_versions ?? [];

	const badProjectName = ["AlreadyExists", "InvalidNameForFolderName"].includes(
		projectNameCheckState,
	);

	const canCreateProject =
		projectNameCheckState !== "checking" && !badProjectName;

	useEffect(() => {
		setUnityVersion(unityVersions[0]);
	}, [unityVersions]);

	const recentProjectLocations = useMemo(() => {
		const copied = [...recentProjectLocationsReversed];
		copied.reverse();
		return copied;
	}, [recentProjectLocationsReversed]);

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
										const disabled =
											!template.available ||
											template.unity_versions.length === 0;
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
					{/*Note that this is an abuse of Select*/}
					<Select value={""} onValueChange={(v) => setProjectLocation(v)}>
						<SelectTrigger>
							<SelectValue placeholder={projectLocation} />
						</SelectTrigger>
						<SelectContent>
							{!recentProjectLocations.includes(lastPickedLocation) && (
								<SelectItem value={lastPickedLocation}>
									{lastPickedLocation}
								</SelectItem>
							)}
							{recentProjectLocations.map((path) => (
								<SelectItem value={path} key={path}>
									{path}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
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
