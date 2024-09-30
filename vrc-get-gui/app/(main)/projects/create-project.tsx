import { VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
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
import { tc, tt } from "@/lib/i18n";
import { pathSeparator } from "@/lib/os";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useFilePickerFunction } from "@/lib/use-file-picker-dialog";
import { useDebounce } from "@uidotdev/usehooks";
import { RefreshCw } from "lucide-react";
import { useRouter } from "next/navigation";
import type React from "react";
import { useId } from "react";
import { useEffect, useState } from "react";

type CreateProjectstate =
	| "loadingInitialInformation"
	| "enteringInformation"
	| "creating";

export function CreateProject({
	close,
	refetch,
}: {
	close?: () => void;
	refetch?: () => void;
}) {
	const router = useRouter();

	const [state, setState] = useState<CreateProjectstate>(
		"loadingInitialInformation",
	);
	const [projectNameCheckState, setProjectNameCheckState] = useState<
		"checking" | TauriProjectDirCheckResult
	>("Ok");

	type CustomTemplate = TauriProjectTemplate & { type: "Custom" };

	const templateUnityVersions = [
		"2022.3.22f1",
		"2022.3.6f1",
		"2019.4.31f1",
	] as const;
	const latestUnityVersion = templateUnityVersions[0];

	type TemplateType = "avatars" | "worlds" | "custom";
	type TemplateUnityVersion = (typeof templateUnityVersions)[number];

	const [customTemplates, setCustomTemplates] = useState<CustomTemplate[]>([]);

	const [templateType, setTemplateType] = useState<TemplateType>("avatars");
	const [unityVersion, setUnityVersion] =
		useState<TemplateUnityVersion>(latestUnityVersion);
	const [customTemplate, setCustomTemplate] = useState<CustomTemplate>();

	function onCustomTemplateChange(value: string) {
		const newCustomTemplate: CustomTemplate = {
			type: "Custom",
			name: value,
		};
		setCustomTemplate(newCustomTemplate);
	}

	const [projectNameRaw, setProjectName] = useState("New Project");
	const projectName = projectNameRaw.trim();
	const [projectLocation, setProjectLocation] = useState("");
	const projectNameDebounced = useDebounce(projectName, 500);

	const [pickProjectDefaultPath, dialog] = useFilePickerFunction(
		commands.environmentPickProjectDefaultPath,
	);

	useEffect(() => {
		(async () => {
			const information =
				await commands.environmentProjectCreationInformation();
			const customTemplates = information.templates.filter(
				(template): template is CustomTemplate => template.type === "Custom",
			);
			setCustomTemplates(customTemplates);
			setCustomTemplate(customTemplates[0]);
			setProjectLocation(information.default_path);
			setState("enteringInformation");
		})();
	}, []);

	useEffect(() => {
		let canceled = false;
		(async () => {
			try {
				setProjectNameCheckState("checking");
				const result = await commands.environmentCheckProjectName(
					projectLocation,
					projectNameDebounced,
				);
				if (canceled) return;
				setProjectNameCheckState(result);
			} catch (e) {
				console.error("Error checking project name", e);
				toastThrownError(e);
			}
		})();
		return () => {
			canceled = true;
		};
	}, [projectNameDebounced, projectLocation]);

	const selectProjectDefaultFolder = async () => {
		try {
			const result = await pickProjectDefaultPath();
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
		} catch (e) {
			console.error(e);
			toastThrownError(e);
		}
	};

	const createProject = async () => {
		try {
			setState("creating");
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
			await commands.environmentCreateProject(
				projectLocation,
				projectName,
				template,
			);
			toastSuccess(tt("projects:toast:project created"));
			close?.();
			refetch?.();
			const projectPath = `${projectLocation}${pathSeparator()}${projectName}`;
			router.push(`/projects/manage?${new URLSearchParams({ projectPath })}`);
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			close?.();
		}
	};

	const checking =
		projectNameDebounced !== projectName ||
		projectNameCheckState === "checking";

	let projectNameState: "Ok" | "warn" | "err";
	let projectNameCheck: React.ReactNode;

	switch (projectNameCheckState) {
		case "Ok":
			projectNameCheck = tc("projects:hint:create project ready");
			projectNameState = "Ok";
			break;
		case "InvalidNameForFolderName":
			projectNameCheck = tc("projects:hint:invalid project name");
			projectNameState = "err";
			break;
		case "MayCompatibilityProblem":
			projectNameCheck = tc("projects:hint:warn symbol in project name");
			projectNameState = "warn";
			break;
		case "WideChar":
			projectNameCheck = tc(
				"projects:hint:warn multibyte char in project name",
			);
			projectNameState = "warn";
			break;
		case "AlreadyExists":
			projectNameCheck = tc("projects:hint:project already exists");
			projectNameState = "err";
			break;
		case "checking":
			projectNameCheck = <RefreshCw className={"w-5 h-5 animate-spin"} />;
			projectNameState = "Ok";
			break;
		default:
			assertNever(projectNameCheckState);
	}

	let projectNameStateClass: React.ReactNode;
	switch (projectNameState) {
		case "Ok":
			projectNameStateClass = "text-success";
			break;
		case "warn":
			projectNameStateClass = "text-warning";
			break;
		case "err":
			projectNameStateClass = "text-destructive";
	}

	if (checking)
		projectNameCheck = <RefreshCw className={"w-5 h-5 animate-spin"} />;

	const inputId = useId();

	let dialogBody: React.ReactNode;

	switch (state) {
		case "loadingInitialInformation":
			dialogBody = <RefreshCw className={"w-5 h-5 animate-spin"} />;
			break;
		case "enteringInformation": {
			const renderUnityVersion = (unityVersion: string) => {
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
			};
			dialogBody = (
				<>
					<VStack>
						<div className={"flex gap-1"}>
							<div className={"flex items-center"}>
								<label htmlFor={inputId}>{tc("projects:template:type")}</label>
							</div>
							<Select
								defaultValue={templateType}
								onValueChange={(value) =>
									setTemplateType(value as TemplateType)
								}
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
							<div className={"flex gap-1"}>
								<div className={"flex items-center"}>
									<label htmlFor={inputId}>
										{tc("projects:template:unity version")}
									</label>
								</div>
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
												{renderUnityVersion(unityVersion)}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
						) : (
							<div className={"flex gap-1"}>
								<div className={"flex items-center"}>
									<label htmlFor={inputId}>{tc("projects:template")}</label>
								</div>
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
						)}
						<Input
							value={projectNameRaw}
							onChange={(e) => setProjectName(e.target.value)}
						/>
						<div className={"flex gap-1 items-center"}>
							<Input className="flex-auto" value={projectLocation} disabled />
							<Button
								className="flex-none px-4"
								onClick={selectProjectDefaultFolder}
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
						<small className={`whitespace-normal ${projectNameStateClass}`}>
							{projectNameCheck}
						</small>
					</VStack>
				</>
			);
			break;
		}
		case "creating":
			dialogBody = (
				<>
					<RefreshCw className={"w-5 h-5 animate-spin"} />
					<p>{tc("projects:creating project...")}</p>
				</>
			);
			break;
	}

	return (
		<DialogOpen>
			<DialogTitle>{tc("projects:create new project")}</DialogTitle>
			<DialogDescription>{dialogBody}</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={close} disabled={state === "creating"}>
					{tc("general:button:cancel")}
				</Button>
				<Button
					onClick={createProject}
					disabled={
						state === "creating" || checking || projectNameState === "err"
					}
				>
					{tc("projects:button:create")}
				</Button>
			</DialogFooter>
			{dialog}
		</DialogOpen>
	);
}
