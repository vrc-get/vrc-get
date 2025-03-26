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
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriProjectDirCheckResult,
	TauriProjectTemplate,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { router } from "@/lib/main";
import { pathSeparator } from "@/lib/os";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useDebounce } from "@uidotdev/usehooks";
import { RefreshCw } from "lucide-react";
import type React from "react";
import { useId } from "react";
import { useState } from "react";

export async function createProject() {
	using dialog = showDialog(<LoadingInitialInformation />);

	const information = await commands.environmentProjectCreationInformation();
	const customTemplates = information.templates.filter(
		(template): template is CustomTemplate => template.type === "Custom",
	);

	const result = await dialog.ask(EnteringInformation, {
		customTemplates,
		projectLocation: information.default_path,
	});

	if (result == null) return;

	dialog.replace(<CreatingProject />);

	await commands.environmentCreateProject(
		result.projectLocation,
		result.projectName,
		result.template,
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

type CustomTemplate = TauriProjectTemplate & { type: "Custom" };

const templateUnityVersions = [
	"2022.3.22f1",
	"2022.3.6f1",
	"2019.4.31f1",
] as const;
const latestUnityVersion = templateUnityVersions[0];

type TemplateType = "avatars" | "worlds" | "custom";
type TemplateUnityVersion = (typeof templateUnityVersions)[number];

interface ProjectCreationInformation {
	template: TauriProjectTemplate;
	projectLocation: string;
	projectName: string;
}

function EnteringInformation({
	customTemplates,
	projectLocation: projectLocationFirst,
	dialog,
}: {
	customTemplates: CustomTemplate[];
	projectLocation: string;
	dialog: DialogContext<null | ProjectCreationInformation>;
}) {
	const [templateType, setTemplateType] = useState<TemplateType>("avatars");
	const [unityVersion, setUnityVersion] =
		useState<TemplateUnityVersion>(latestUnityVersion);
	const [customTemplate, setCustomTemplate] = useState<
		CustomTemplate | undefined
	>(customTemplates[0]);

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
		let template: TauriProjectTemplate;
		switch (templateType) {
			case "avatars":
			case "worlds":
				template = {
					type: "Builtin",
					id: `${templateType}-${unityVersion}`,
					name: `${templateType}-${unityVersion}`,
				};
				break;
			case "custom":
				if (customTemplate === undefined)
					throw new Error("Custom template not selected");
				template = customTemplate;
				break;
			default:
				assertNever(templateType, "template type");
		}
		dialog.close({
			template,
			projectLocation,
			projectName,
		});
	};

	const inputId = useId();

	const badProjectName = ["AlreadyExists", "InvalidNameForFolderName"].includes(
		projectNameCheckState,
	);

	const canCreateProject =
		projectNameCheckState !== "checking" && !badProjectName;

	return (
		<DialogBase
			close={() => dialog.close(null)}
			createProject={canCreateProject ? createProject : undefined}
		>
			<VStack>
				<div className={"flex gap-1"}>
					<div className={"flex items-center"}>
						<label htmlFor={inputId}>{tc("projects:template:type")}</label>
					</div>
					<Select
						defaultValue={templateType}
						onValueChange={(value) => setTemplateType(value as TemplateType)}
					>
						<SelectTrigger id={inputId}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectGroup>
								<SelectItem value={"avatars"}>
									{tc("projects:type:avatars")}
								</SelectItem>
								<SelectItem value={"worlds"}>
									{tc("projects:type:worlds")}
								</SelectItem>
								<SelectItem
									value={"custom"}
									disabled={customTemplates.length === 0}
								>
									{tc("projects:type:custom")}
								</SelectItem>
							</SelectGroup>
						</SelectContent>
					</Select>
				</div>
				{templateType !== "custom" ? (
					<BuiltinTemplateSelection
						templateUnityVersions={templateUnityVersions}
						unityVersion={unityVersion}
						setUnityVersion={setUnityVersion}
					/>
				) : (
					<CustomTemplateSelection
						customTemplates={customTemplates}
						customTemplate={customTemplate}
						setCustomTemplate={setCustomTemplate}
					/>
				)}
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

function BuiltinTemplateSelection({
	unityVersion,
	templateUnityVersions,
	setUnityVersion,
}: {
	unityVersion: TemplateUnityVersion;
	templateUnityVersions: readonly TemplateUnityVersion[];
	setUnityVersion: (unity: TemplateUnityVersion) => void;
}) {
	const inputId = useId();

	return (
		<div className={"flex items-center gap-1 whitespace-nowrap"}>
			<label htmlFor={inputId}>{tc("projects:template:unity version")}</label>
			<Select
				defaultValue={unityVersion}
				onValueChange={(value) =>
					setUnityVersion(value as TemplateUnityVersion)
				}
			>
				<SelectTrigger id={inputId}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					{templateUnityVersions.map((unityVersion) => (
						<SelectItem value={unityVersion} key={unityVersion}>
							<UnityVersion
								unityVersion={unityVersion}
								latestUnityVersion={latestUnityVersion}
							/>
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</div>
	);
}

function CustomTemplateSelection({
	customTemplates,
	customTemplate,
	setCustomTemplate,
}: {
	customTemplates: readonly CustomTemplate[];
	customTemplate: CustomTemplate | undefined;
	setCustomTemplate: (value: CustomTemplate) => void;
}) {
	function onCustomTemplateChange(value: string) {
		const newCustomTemplate: CustomTemplate = {
			type: "Custom",
			name: value,
		};
		setCustomTemplate(newCustomTemplate);
	}

	const inputId = useId();
	return (
		<div className={"flex items-center gap-1 whitespace-nowrap"}>
			<label htmlFor={inputId}>{tc("projects:template")}</label>
			<Select
				value={customTemplate?.name}
				onValueChange={onCustomTemplateChange}
			>
				<SelectTrigger id={inputId}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{customTemplates.map((template) => (
							<SelectItem value={template.name} key={template.name}>
								{template.name}
							</SelectItem>
						))}
					</SelectGroup>
				</SelectContent>
			</Select>
		</div>
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
