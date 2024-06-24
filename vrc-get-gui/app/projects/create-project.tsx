import {useRouter} from "next/navigation";
import React, {useEffect, useState} from "react";
import {
  environmentCheckProjectName, environmentCreateProject,
  environmentPickProjectDefaultPath,
  environmentProjectCreationInformation,
  TauriProjectDirCheckResult,
  TauriProjectTemplate
} from "@/lib/bindings";
import {useDebounce} from "@uidotdev/usehooks";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";
import {pathSeparator} from "@/lib/os";
import {ArrowPathIcon} from "@heroicons/react/24/solid";
import {VStack} from "@/components/layout";
import {Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue} from "@/components/ui/select";
import {Input} from "@/components/ui/input";
import {Button} from "@/components/ui/button";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";

type CreateProjectstate = 'loadingInitialInformation' | 'enteringInformation' | 'creating';

export function CreateProject(
  {
    close,
    refetch,
  }: {
    close?: () => void,
    refetch?: () => void,
  }
) {
  const router = useRouter();

  const [state, setState] = useState<CreateProjectstate>('loadingInitialInformation');
  const [projectNameCheckState, setProjectNameCheckState] = useState<'checking' | TauriProjectDirCheckResult>('Ok');

  type CustomTemplate = TauriProjectTemplate & { type: 'Custom' };

  const templateUnityVersions = [
    '2022.3.22f1',
    '2022.3.6f1',
    '2019.4.31f1',
  ] as const;
  const latestUnityVersion = templateUnityVersions[0];

  const [customTemplates, setCustomTemplates] = useState<CustomTemplate[]>([]);

  const [templateType, setTemplateType] = useState<'avatars' | 'worlds' | 'custom'>('avatars');
  const [unityVersion, setUnityVersion] = useState<(typeof templateUnityVersions)[number]>(latestUnityVersion);
  const [customTemplate, setCustomTemplate] = useState<CustomTemplate>();

  function onCustomTemplateChange(value: string) {
    let newCustomTemplate: CustomTemplate = {
      type: "Custom",
      name: value,
    }
    setCustomTemplate(newCustomTemplate);
  }

  const [projectNameRaw, setProjectName] = useState("New Project");
  const projectName = projectNameRaw.trim();
  const [projectLocation, setProjectLocation] = useState("");
  const projectNameDebounced = useDebounce(projectName, 500);

  const [pickProjectDefaultPath, dialog] = useFilePickerFunction(environmentPickProjectDefaultPath);

  useEffect(() => {
    (async () => {
      const information = await environmentProjectCreationInformation();
      const customTemplates = information.templates.filter((template): template is CustomTemplate => template.type === "Custom");
      setCustomTemplates(customTemplates);
      setCustomTemplate(customTemplates[0]);
      setProjectLocation(information.default_path);
      setState('enteringInformation');
    })();
  }, []);

  useEffect(() => {
    let canceled = false;
    (async () => {
      try {
        setProjectNameCheckState('checking');
        const result = await environmentCheckProjectName(projectLocation, projectNameDebounced);
        if (canceled) return;
        setProjectNameCheckState(result);
      } catch (e) {
        console.error("Error checking project name", e);
        toastThrownError(e);
      }
    })()
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
          const _exhaustiveCheck: never = result;
      }
    } catch (e) {
      console.error(e);
      toastThrownError(e)
    }
  };

  const createProject = async () => {
    try {
      setState('creating');
      let template: TauriProjectTemplate;
      switch (templateType) {
        case "avatars":
        case "worlds":
          template = {
            type: "Builtin",
            id: `${templateType}-${unityVersion}`,
            name: `${templateType}-${unityVersion}`,
          }
          break;
        case "custom":
          if (customTemplate === undefined)
            throw new Error("Custom template not selected");
          template = customTemplate;
          break;
        default:
          const _exhaustiveCheck: never = templateType;
          template = _exhaustiveCheck;
          break;
      }
      await environmentCreateProject(projectLocation, projectName, template);
      toastSuccess(tt("projects:toast:project created"));
      close?.();
      refetch?.();
      const projectPath = `${projectLocation}${pathSeparator()}${projectName}`;
      router.push(`/projects/manage?${new URLSearchParams({projectPath})}`);
    } catch (e) {
      console.error(e);
      toastThrownError(e);
      close?.();
    }
  };

  const checking = projectNameDebounced != projectName || projectNameCheckState === "checking";

  let projectNameState: 'Ok' | 'warn' | 'err';
  let projectNameCheck;

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
      projectNameCheck = tc("projects:hint:warn multibyte char in project name");
      projectNameState = "warn";
      break;
    case "AlreadyExists":
      projectNameCheck = tc("projects:hint:project already exists");
      projectNameState = "err";
      break;
    case "checking":
      projectNameCheck = <ArrowPathIcon className={"w-5 h-5 animate-spin"} />;
      projectNameState = "Ok";
      break;
    default:
      const _exhaustiveCheck: never = projectNameCheckState;
      projectNameState = "err";
  }

  let projectNameStateClass;
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

  if (checking) projectNameCheck = <ArrowPathIcon className={"w-5 h-5 animate-spin"} />

  let dialogBody;

  switch (state) {
    case "loadingInitialInformation":
      dialogBody = <ArrowPathIcon className={"w-5 h-5 animate-spin"} />;
      break;
    case "enteringInformation":
      const renderUnityVersion = (unityVersion: string) => {
        if (unityVersion === latestUnityVersion) {
          return <>{unityVersion} <span className={"text-success"}>{tc("projects:latest")}</span></>
        } else {
          return unityVersion;
        }
      }
      dialogBody = <>
        <VStack>
          <div className={"flex gap-1"}>
            <div className={"flex items-center"}>
              <label>{tc("projects:template:type")}</label>
            </div>
            <Select defaultValue={templateType} onValueChange={value => setTemplateType(value as any)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value={"avatars"}>{tc("projects:type:avatars")}</SelectItem>
                  <SelectItem value={"worlds"}>{tc("projects:type:worlds")}</SelectItem>
                  <SelectItem value={"custom"} disabled={customTemplates.length == 0}>{tc("projects:type:custom")}</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          {templateType !== "custom" ? (
            <div className={"flex gap-1"}>
              <div className={"flex items-center"}>
                <label>{tc("projects:template:unity version")}</label>
              </div>
              <Select defaultValue={unityVersion} onValueChange={value => setUnityVersion(value as any)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {templateUnityVersions.map(unityVersion =>
                    <SelectItem value={unityVersion} key={unityVersion}>{renderUnityVersion(unityVersion)}</SelectItem>)}
                </SelectContent>
              </Select>
            </div>
          ) : (
            <div className={"flex gap-1"}>
              <div className={"flex items-center"}>
                <label>{tc("projects:template")}</label>
              </div>
              <Select value={customTemplate?.name} onValueChange={onCustomTemplateChange}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {customTemplates.map(template =>
                      <SelectItem value={template.name} key={template.name}>{template.name}</SelectItem>)}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
          )}
          <Input value={projectNameRaw} onChange={(e) => setProjectName(e.target.value)}/>
          <div className={"flex gap-1 items-center"}>
            <Input className="flex-auto" value={projectLocation} disabled/>
            <Button className="flex-none px-4"
                    onClick={selectProjectDefaultFolder}>{tc("general:button:select")}</Button>
          </div>
          <small className={"whitespace-normal"}>
            {tc("projects:hint:path of creating project", {path: `${projectLocation}${pathSeparator()}${projectName}`}, {
              components: {
                path: <span className={"p-0.5 font-path whitespace-pre bg-secondary text-secondary-foreground"}/>
              }
            })}
          </small>
          <small className={`whitespace-normal ${projectNameStateClass}`}>
            {projectNameCheck}
          </small>
        </VStack>
      </>;
      break;
    case "creating":
      dialogBody = <>
        <ArrowPathIcon className={"w-5 h-5 animate-spin"} />
        <p>{tc("projects:creating project...")}</p>
      </>;
      break;
  }

  return <DialogOpen>
    <DialogTitle>{tc("projects:create new project")}</DialogTitle>
    <DialogDescription>
      {dialogBody}
    </DialogDescription>
    <DialogFooter className={"gap-2"}>
      <Button onClick={close} disabled={state == "creating"}>{tc("general:button:cancel")}</Button>
      <Button onClick={createProject}
              disabled={state == "creating" || checking || projectNameState == "err"}>{tc("projects:button:create")}</Button>
    </DialogFooter>
    {dialog}
  </DialogOpen>;
}
