"use client"

import {Button} from "@/components/ui/button";
import {Card, CardHeader} from "@/components/ui/card";
import {
	Checkbox,
	Dialog,
	DialogBody,
	DialogFooter,
	DialogHeader,
	Input,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Spinner,
	Tooltip,
} from "@material-tailwind/react";
import React, {forwardRef, Fragment, useEffect, useMemo, useState} from "react";
import {
	ArrowPathIcon,
	ChevronDownIcon,
	ChevronUpDownIcon,
	EllipsisHorizontalIcon,
	GlobeAltIcon,
	QuestionMarkCircleIcon,
	StarIcon,
	UserCircleIcon
} from "@heroicons/react/24/solid";
import {HNavBar, VStack} from "@/components/layout";
import {
	environmentAddProjectWithPicker,
	environmentCheckProjectName,
	environmentCopyProjectForMigration,
	environmentCreateProject,
	environmentGetProjectSorting,
	environmentPickProjectDefaultPath,
	environmentProjectCreationInformation,
	environmentProjects,
	environmentSetFavoriteProject,
	environmentSetProjectSorting,
	environmentUnityVersions,
	projectMigrateProjectToVpm,
	TauriProject,
	TauriProjectDirCheckResult,
	TauriProjectTemplate,
	TauriProjectType,
	utilOpen
} from "@/lib/bindings";
import {useQuery} from "@tanstack/react-query";
import {useRouter} from "next/navigation";
import {SearchBox} from "@/components/SearchBox";
import {nop} from "@/lib/nop";
import {useDebounce} from "@uidotdev/usehooks";
import {VGOption, VGSelect} from "@/components/select";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {useRemoveProjectModal} from "@/lib/remove-project";
import {tc, tt} from "@/lib/i18n";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {pathSeparator} from "@/lib/os";
import {useBackupProjectModal} from "@/lib/backup-project";
import {ChevronUpIcon} from "@heroicons/react/24/outline";
import {compareUnityVersionString} from "@/lib/version";
import {useOpenUnity, OpenUnityFunction} from "@/lib/use-open-unity";

const sortings = [
	"lastModified",
	"name",
	"unity",
	"type",
] as const;

type SimpleSorting = (typeof sortings)[number];
type Sorting = SimpleSorting | `${SimpleSorting}Reversed`;

function isSorting(s: string): s is Sorting {
	return sortings.some(sorting => sorting === s || `${sorting}Reversed` === s);
}

export default function Page() {
	const result = useQuery({
		queryKey: ["projects"],
		queryFn: environmentProjects,
	});
	const unityVersionsResult = useQuery({
		queryKey: ["unityVersions"],
		queryFn: () => environmentUnityVersions(),
	});

	const [search, setSearch] = useState("");
	const [loadingOther, setLoadingOther] = useState(false);
	const [createProjectState, setCreateProjectState] = useState<'normal' | 'creating'>('normal');
	const openUnity = useOpenUnity(unityVersionsResult?.data);

	const startCreateProject = () => setCreateProjectState('creating');

	const loading = result.isFetching || loadingOther;

	return (
		<VStack className={"m-4"}>
			<ProjectViewHeader className={"flex-shrink-0"}
												 refresh={() => result.refetch()}
												 startCreateProject={startCreateProject}
												 isLoading={loading}
												 search={search} setSearch={setSearch}/>
			<main className="flex-shrink overflow-hidden flex">
				<Card className="w-full overflow-x-auto overflow-y-auto shadow-none">
          <CardHeader>
            {
              result.status == "pending" ? <Card className={"p-4"}>{tc("general:loading...")}</Card> :
                result.status == "error" ?
                  <Card className={"p-4"}>{tc("projects:error:load error", {msg: result.error.message})}</Card> :
                  <ProjectsTable
                    projects={result.data}
                    search={search}
                    loading={loading}
                    openUnity={openUnity.openUnity}
                    refresh={() => result.refetch()}
                    onRemoved={() => result.refetch()}
                  />
            }
          </CardHeader>
				</Card>
				{createProjectState === "creating" &&
					<CreateProject close={() => setCreateProjectState("normal")} refetch={() => result.refetch()}/>}
				{openUnity.dialog}
			</main>
		</VStack>
	);
}

function compareProjectType(a: TauriProjectType, b: TauriProjectType): 0 | -1 | 1 {
	if (a === b) return 0;

	// legacy unknown
	if (a === "LegacySdk2") return 1;
	if (b === "LegacySdk2") return -1;
	if (a === "UpmStarter") return 1;
	if (b === "UpmStarter") return -1;

	// legacy worlds 
	if (a === "LegacyWorlds") return 1;
	if (b === "LegacyWorlds") return -1;
	if (a === "UpmWorlds") return 1;
	if (b === "UpmWorlds") return -1;

	// legacy avatars
	if (a === "LegacyAvatars") return 1;
	if (b === "LegacyAvatars") return -1;
	if (a === "UpmAvatars") return 1;
	if (b === "UpmAvatars") return -1;

	// unknown
	if (a === "Unknown") return 1;
	if (b === "Unknown") return -1;
	if (a === "VpmStarter") return 1;
	if (b === "VpmStarter") return -1;

	// worlds
	if (a === "Worlds") return 1;
	if (b === "Worlds") return -1;

	// avatars
	if (a === "Avatars") return 1;
	if (b === "Avatars") return -1;

	let _: never = a;
	return 0;
}

function ProjectsTable(
	{
		projects, search, onRemoved, loading, refresh, openUnity,
	}: {
		projects: TauriProject[],
		openUnity: OpenUnityFunction,
		search?: string,
		loading?: boolean,
		onRemoved?: () => void;
		refresh?: () => void,
	}
) {
	const [sorting, setSortingState] = useState<Sorting>("lastModified");

	useEffect(() => {
		(async () => {
			let newSorting = await environmentGetProjectSorting();
			if (newSorting === null) newSorting = "lastModified";
			if (!isSorting(newSorting)) {
				setSortingState("lastModified");
			} else {
				setSortingState(newSorting);
			}
		})()
	}, []);

	const projectsShown = useMemo(() => {
		let searched = projects.filter(project => project.name.toLowerCase().includes(search?.toLowerCase() ?? ""));
		searched.sort((a, b) => b.last_modified - a.last_modified);
		switch (sorting) {
			case "lastModified":
				// already sorted
				break;
			case "lastModifiedReversed":
				searched.sort((a, b) => a.last_modified - b.last_modified);
				break;
			case "name":
				searched.sort((a, b) => a.name.localeCompare(b.name));
				break;
			case "nameReversed":
				searched.sort((a, b) => b.name.localeCompare(a.name));
				break;
			case "type":
				searched.sort((a, b) => compareProjectType(a.project_type, b.project_type));
				break;
			case "typeReversed":
				searched.sort((a, b) => compareProjectType(b.project_type, a.project_type));
				break;
			case "unity":
				searched.sort((a, b) => compareUnityVersionString(a.unity, b.unity));
				break;
			case "unityReversed":
				searched.sort((a, b) => compareUnityVersionString(b.unity, a.unity));
				break;
			default:
				let _: never = sorting;
		}
		searched.sort((a, b) => {
			if (a.favorite && !b.favorite) return -1;
			if (!a.favorite && b.favorite) return 1;
			return 0;
		})
		return searched;
	}, [projects, sorting, search]);

	const thClass = `sticky top-0 z-10 border-b border-blue-gray-100 p-2.5`;
	const iconClass = `size-3 invisible project-table-header-chevron-up-down`;

	const setSorting = async (simpleSorting: SimpleSorting) => {
		let newSorting: Sorting;
		if (sorting === simpleSorting) {
			newSorting = `${simpleSorting}Reversed`;
		} else if (sorting === `${simpleSorting}Reversed`) {
			newSorting = simpleSorting;
		} else {
			newSorting = simpleSorting;
		}
		setSortingState(newSorting);

		try {
			await environmentSetProjectSorting(newSorting);
		} catch (e) {
			console.error("Error setting project sorting", e);
			toastThrownError(e);
		}
	}

	const headerBg = (target: SimpleSorting) => sorting === target || sorting === `${target}Reversed` ? "bg-blue-100" : "bg-blue-gray-50";
	const icon = (target: SimpleSorting) =>
		sorting === target ? <ChevronDownIcon className={"size-3"}/>
			: sorting === `${target}Reversed` ? <ChevronUpIcon className={"size-3"}/>
				: <ChevronUpDownIcon className={iconClass}/>;

	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				<th className={`${thClass} bg-blue-gray-50`}>
					<StarIcon className={"size-4"}/>
				</th>
				<th
					className={`${thClass} ${headerBg('name')}`}>
					<button className={"flex w-full project-table-button"}
									onClick={() => setSorting("name")}>
						{icon("name")}
						<small className="font-normal leading-none">{tc("general:name")}</small>
					</button>
				</th>
				<th
					className={`${thClass} ${headerBg('type')}`}>
					<button className={"flex w-full project-table-button"} onClick={() => setSorting("type")}>
						{icon("type")}
						<small className="font-normal leading-none">{tc("projects:type")}</small>
					</button>
				</th>
				<th
					className={`${thClass} ${headerBg('unity')}`}>
					<button className={"flex w-full project-table-button"} onClick={() => setSorting("unity")}>
						{icon("unity")}
						<small className="font-normal leading-none">{tc("projects:unity")}</small>
					</button>
				</th>
				<th
					className={`${thClass} ${headerBg('lastModified')}`}>
					<button className={"flex w-full project-table-button"} onClick={() => setSorting("lastModified")}>
						{icon("lastModified")}
						<small className="font-normal leading-none">{tc("projects:last modified")}</small>
					</button>
				</th>
				<th className={`${thClass} bg-blue-gray-50`}></th>
			</tr>
			</thead>
			<tbody>
			{projectsShown.map((project) =>
				<ProjectRow key={project.index} project={project} loading={loading} refresh={refresh} onRemoved={onRemoved}
										openUnity={openUnity}/>)}
			</tbody>
		</table>
	);
}

const ProjectDisplayType: Record<TauriProjectType, "avatars" | "worlds" | "sdk2" | "unknown"> = {
	"Unknown": "unknown",
	"LegacySdk2": "sdk2",
	"LegacyWorlds": "worlds",
	"LegacyAvatars": "avatars",
	"UpmWorlds": "worlds",
	"UpmAvatars": "avatars",
	"UpmStarter": "unknown",
	"Worlds": "worlds",
	"Avatars": "avatars",
	"VpmStarter": "unknown",
}

const LegacyProjectTypes = ["LegacySdk2", "LegacyWorlds", "LegacyAvatars", "UpmWorlds", "UpmAvatars", "UpmStarter"];

function formatDateOffset(date: number): React.ReactNode {
	const now = Date.now();
	const diff = now - date;

	const PER_SECOND = 1000;
	const PER_MINUTE = 60 * PER_SECOND;
	const PER_HOUR = 60 * PER_MINUTE;
	const PER_DAY = 24 * PER_HOUR;
	const PER_WEEK = 7 * PER_DAY;
	const PER_MONTH = 30 * PER_DAY;
	const PER_YEAR = 365 * PER_DAY;

	const diffAbs = Math.abs(diff);

	if (diffAbs < PER_MINUTE) return tc("projects:last modified:moments");
	if (diffAbs < PER_HOUR) return tc("projects:last modified:minutes", {count: Math.floor(diff / PER_MINUTE)});
	if (diffAbs < PER_DAY) return tc("projects:last modified:hours", {count: Math.floor(diff / PER_HOUR)});
	if (diffAbs < PER_WEEK) return tc("projects:last modified:days", {count: Math.floor(diff / PER_DAY)});
	if (diffAbs < PER_MONTH) return tc("projects:last modified:weeks", {count: Math.floor(diff / PER_WEEK)});
	if (diffAbs < PER_YEAR) return tc("projects:last modified:months", {count: Math.floor(diff / PER_MONTH)});

	return tc("projects:last modified:years", {count: Math.floor(diff / PER_YEAR)});
}

type ProjectRowState = {
	type: 'normal',
} | {
	type: 'migrateVpm:confirm',
} | {
	type: 'migrateVpm:copyingProject',
} | {
	type: 'migrateVpm:updating',
}

function ProjectRow(
	{
		project,
		openUnity,
		onRemoved,
		loading,
		refresh,
	}: {
		project: TauriProject;
		openUnity: OpenUnityFunction;
		onRemoved?: () => void;
		loading?: boolean;
		refresh?: () => void;
	}
) {
	const router = useRouter();

	const [dialogStatus, setDialogStatus] = useState<ProjectRowState>({type: 'normal'});
	const removeProjectModal = useRemoveProjectModal({onRemoved});
	const backupProjectModal = useBackupProjectModal();

	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const typeIconClass = `w-5 h-5`;

	const projectTypeKind = ProjectDisplayType[project.project_type] ?? "unknown";
	const displayType = tc(`projects:type:${projectTypeKind}`)
	const isLegacy = LegacyProjectTypes.includes(project.project_type);
	const lastModified = new Date(project.last_modified);
	const lastModifiedHumanReadable = `${lastModified.getFullYear().toString().padStart(4, '0')}-${(lastModified.getMonth() + 1).toString().padStart(2, '0')}-${lastModified.getDate().toString().padStart(2, '0')} ${lastModified.getHours().toString().padStart(2, "0")}:${lastModified.getMinutes().toString().padStart(2, "0")}:${lastModified.getSeconds().toString().padStart(2, "0")}`;

	const openProjectFolder = () => utilOpen(project.path);

	const startMigrateVpm = () => setDialogStatus({type: 'migrateVpm:confirm'});
	const doMigrateVpm = async (inPlace: boolean) => {
		setDialogStatus({type: 'normal'});
		try {
			let migrateProjectPath;
			if (inPlace) {
				migrateProjectPath = project.path;
			} else {
				// copy
				setDialogStatus({type: "migrateVpm:copyingProject"});
				migrateProjectPath = await environmentCopyProjectForMigration(project.path);
			}
			setDialogStatus({type: "migrateVpm:updating"});
			await projectMigrateProjectToVpm(migrateProjectPath);
			setDialogStatus({type: "normal"});
			toastSuccess(tt("projects:toast:project migrated"));
			refresh?.();
		} catch (e) {
			console.error("Error migrating project", e);
			setDialogStatus({type: "normal"});
			toastThrownError(e);
		}
	}

	const onToggleFavorite = async () => {
		try {
			await environmentSetFavoriteProject(project.list_version, project.index, !project.favorite);
			refresh?.();
		} catch (e) {
			console.error("Error migrating project", e);
			toastThrownError(e);
		}
	}

	const removed = !project.is_exists;

	const MayTooltip = removed ? Tooltip : Fragment;

	const RowButton = forwardRef<HTMLButtonElement, React.ComponentProps<typeof Button>>(function RowButton(props, ref) {
		if (removed) {
			return <Tooltip content={tt("projects:tooltip:no directory")}>
				<Button {...props} className={`disabled:pointer-events-auto ${props.className}`} disabled ref={ref}/>
			</Tooltip>
		} else {
			return (
				<Button {...props} className={`disabled:pointer-events-auto ${props.className}`}
								disabled={loading || props.disabled} ref={ref}/>
			);
		}
	});

	let manageButton;

	switch (project.project_type) {
		case "LegacySdk2":
			manageButton =
				<Tooltip content={tc("projects:tooltip:sdk2 migration hint")}>
					<RowButton variant={"success"} disabled>
						{tc("projects:button:migrate")}
					</RowButton>
				</Tooltip>
			break;
		case "LegacyWorlds":
		case "LegacyAvatars":
			manageButton =
				<RowButton variant={"success"} onClick={startMigrateVpm}>{tc("projects:button:migrate")}</RowButton>
			break;
		case "UpmWorlds":
		case "UpmAvatars":
		case "UpmStarter":
			manageButton = <Tooltip content={tc("projects:tooltip:git-vcc not supported")}>
				<RowButton variant={"info"} disabled>
					{tc("projects:button:manage")}
				</RowButton>
			</Tooltip>
			break;
		case "Unknown":
		case "Worlds":
		case "Avatars":
		case "VpmStarter":
			manageButton = <RowButton
				onClick={() => router.push(`/projects/manage?${new URLSearchParams({projectPath: project.path})}`)}
				variant={"info"}>
				{tc("projects:button:manage")}
			</RowButton>
			break;
	}

	let dialogContent: React.ReactNode = null;
	switch (dialogStatus.type) {
		case "migrateVpm:confirm":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("projects:dialog:vpm migrate header")}</DialogHeader>
					<DialogBody>
						<p className={"text-red-700"}>
							{tc("projects:dialog:vpm migrate description")}
						</p>
					</DialogBody>
					<DialogFooter>
						<Button onClick={() => setDialogStatus({type: "normal"})}
										className="mr-1">{tc("general:button:cancel")}</Button>
						<Button onClick={() => doMigrateVpm(false)} variant={"destructive"}
										className="mr-1">{tc("projects:button:migrate copy")}</Button>
						<Button onClick={() => doMigrateVpm(true)} variant={"destructive"}>{tc("projects:button:migrate in-place")}</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		case "migrateVpm:copyingProject":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("projects:dialog:vpm migrate header")}</DialogHeader>
					<DialogBody>
						<p>
							{tc("projects:pre-migrate copying...")}
						</p>
					</DialogBody>
				</Dialog>
			);
			break;
		case "migrateVpm:updating":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("projects:dialog:vpm migrate header")}</DialogHeader>
					<DialogBody>
						<p>
							{tc("projects:migrating...")}
						</p>
					</DialogBody>
				</Dialog>
			);
			break;
	}

	return (
		<tr className={`even:bg-blue-gray-50/50 ${(removed || loading) ? 'opacity-50' : ''}`}>
			<td className={cellClass}>
				<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}}
									checked={project.favorite}
									onChange={onToggleFavorite}
									disabled={removed || loading}
									icon={<StarIcon className={"size-3"}/>}
									className="hover:before:content-none before:transition-none border-none"/>
			</td>
			<td className={cellClass}>
				<MayTooltip content={tc("projects:tooltip:no directory")}>
					<div className="flex flex-col">
						<p className="font-normal whitespace-pre">
							{project.name}
						</p>
						<p className="font-normal opacity-50 text-sm whitespace-pre">
							{project.path}
						</p>
					</div>
				</MayTooltip>
			</td>
			<td className={`${cellClass} w-[8em]`}>
				<div className="flex flex-row gap-2">
					<div className="flex items-center">
						{projectTypeKind === "avatars" ? <UserCircleIcon className={typeIconClass}/> :
							projectTypeKind === "worlds" ? <GlobeAltIcon className={typeIconClass}/> :
								<QuestionMarkCircleIcon className={typeIconClass}/>}
					</div>
					<div className="flex flex-col justify-center">
						<p className="font-normal">
							{displayType}
						</p>
						{isLegacy &&
							<p
								className="font-normal opacity-50 text-sm text-red-700">{tc("projects:type:legacy")}</p>}
					</div>
				</div>
			</td>
			<td className={noGrowCellClass}>
				<p className="font-normal">
					{project.unity}
				</p>
			</td>
			<td className={noGrowCellClass}>
				<Tooltip content={lastModifiedHumanReadable}>
					<time dateTime={lastModified.toISOString()}>
						<time className="font-normal">
							{formatDateOffset(project.last_modified)}
						</time>
					</time>
				</Tooltip>
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					<RowButton
						onClick={() => openUnity(project.path, project.unity, project.unity_revision)}>{tc("projects:button:open unity")}</RowButton>
					{manageButton}
					<RowButton onClick={() => backupProjectModal.startBackup(project)}
										 variant={"success"}>{tc("projects:backup")}</RowButton>
					<Menu>
						<MenuHandler>
							<Button variant="ghost" className={"hover:bg-info/10 hover:text-info text-info"}><EllipsisHorizontalIcon
								className={"size-5"}/></Button>
						</MenuHandler>
						<MenuList>
							<MenuItem onClick={openProjectFolder}
												disabled={removed || loading}>{tc("projects:menuitem:open directory")}</MenuItem>
							<MenuItem onClick={() => removeProjectModal.startRemove(project)} disabled={loading}
												className={'text-red-700 focus:text-red-700'}>
								{tc("projects:remove project")}
							</MenuItem>
						</MenuList>
					</Menu>
				</div>
				{dialogContent}
				{removeProjectModal.dialog}
				{backupProjectModal.dialog}
			</td>
		</tr>
	)
}

function ProjectViewHeader({className, refresh, startCreateProject, isLoading, search, setSearch}: {
	className?: string,
	refresh?: () => void,
	startCreateProject?: () => void
	isLoading?: boolean,
	search: string,
	setSearch: (search: string) => void
}) {
	const [addProjectWithPicker, dialog] = useFilePickerFunction(environmentAddProjectWithPicker);

	const addProject = async () => {
		try {
			const result = await addProjectWithPicker();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("projects:toast:project added"));
					refresh?.();
					break;
				case "AlreadyAdded":
					toastError(tt("projects:toast:project already exists"));
					break;
				default:
					let _: never = result;
			}
		} catch (e) {
			console.error("Error adding project", e);
			toastThrownError(e);
		}
	};

	return (
		<HNavBar className={className}>
			<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
				{tc("projects")}

			<Tooltip content={tc("projects:tooltip:refresh")}>
				<Button variant={"ghost"} onClick={() => refresh?.()} disabled={isLoading}>
					{isLoading ? <Spinner className="w-5 h-5"/> : <ArrowPathIcon className={"w-5 h-5"}/>}
				</Button>
			</Tooltip>
			</p>

			<SearchBox className={"w-max flex-grow"} value={search} onChange={(e) => setSearch(e.target.value)}/>

			<Menu>
        <div className={"flex divide-x"}>
          <Button className={"rounded-r-none pl-4 pr-3"} onClick={startCreateProject}>{tc("projects:create new project")}</Button>
          <MenuHandler className={"rounded-l-none pl-2 pr-2"}>
            <Button>
              <ChevronDownIcon className={"w-4 h-4"}/>
            </Button>
          </MenuHandler>
        </div>
				<MenuList>
					<MenuItem onClick={addProject}>{tc("projects:add existing project")}</MenuItem>
				</MenuList>
			</Menu>

			{dialog}
		</HNavBar>
	);
}

type CreateProjectstate = 'loadingInitialInformation' | 'enteringInformation' | 'creating';

function CreateProject(
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
			projectNameCheck = <Spinner/>;
			projectNameState = "Ok";
			break;
		default:
			const _exhaustiveCheck: never = projectNameCheckState;
			projectNameState = "err";
	}

	let projectNameStateClass;
	switch (projectNameState) {
		case "Ok":
			projectNameStateClass = "text-green-700";
			break;
		case "warn":
			projectNameStateClass = "text-yellow-900";
			break;
		case "err":
			projectNameStateClass = "text-red-900";
	}

	if (checking) projectNameCheck = <Spinner/>

	let dialogBody;

	switch (state) {
		case "loadingInitialInformation":
			dialogBody = <Spinner/>;
			break;
		case "enteringInformation":
			const renderUnityVersion = (unityVersion: string) => {
				if (unityVersion === latestUnityVersion) {
					return <>{unityVersion} <span className={"text-green-700"}>{tc("projects:latest")}</span></>
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
						<VGSelect menuClassName={"z-[19999]"} value={tc(`projects:type:${templateType}`)}
											onChange={value => setTemplateType(value)}>
							<VGOption value={"avatars"}>{tc("projects:type:avatars")}</VGOption>
							<VGOption value={"worlds"}>{tc("projects:type:worlds")}</VGOption>
							<VGOption value={"custom"} disabled={customTemplates.length == 0}>{tc("projects:type:custom")}</VGOption>
						</VGSelect>
					</div>
					{templateType !== "custom" ? (
						<div className={"flex gap-1"}>
							<div className={"flex items-center"}>
								<label>{tc("projects:template:unity version")}</label>
							</div>
							<VGSelect menuClassName={"z-[19999]"} value={renderUnityVersion(unityVersion)}
												onChange={value => setUnityVersion(value)}>
								{templateUnityVersions.map(unityVersion =>
									<VGOption value={unityVersion} key={unityVersion}>{renderUnityVersion(unityVersion)}</VGOption>)}
							</VGSelect>
						</div>
					) : (
						<div className={"flex gap-1"}>
							<div className={"flex items-center"}>
								<label>{tc("projects:template")}</label>
							</div>
							<VGSelect menuClassName={"z-[19999]"} value={customTemplate?.name}
												onChange={value => setCustomTemplate(value)}>
								{customTemplates.map(template =>
									<VGOption value={template} key={template.name}>{template.name}</VGOption>)}
							</VGSelect>
						</div>
					)}
					<Input label={"Project Name"} value={projectNameRaw} onChange={(e) => setProjectName(e.target.value)}/>
					<div className={"flex gap-1"}>
						<Input className="flex-auto" label={"Project Location"} value={projectLocation} disabled/>
						<Button className="flex-none px-4"
										onClick={selectProjectDefaultFolder}>{tc("general:button:select")}</Button>
					</div>
					<small className={"whitespace-normal"}>
						{tc("projects:hint:path of creating project", {path: `${projectLocation}${pathSeparator()}${projectName}`}, {
							components: {
								path: <span className={"p-0.5 font-path whitespace-pre bg-gray-100"}/>
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
				<Spinner/>
				<p>{tc("projects:creating project...")}</p>
			</>;
			break;
	}

	return <Dialog handler={nop} open>
		<DialogHeader>{tc("projects:create new project")}</DialogHeader>
		<DialogBody>
			{dialogBody}
		</DialogBody>
		<DialogFooter>
			<div className={"flex gap-2"}>
				<Button onClick={close} disabled={state == "creating"}>{tc("general:button:cancel")}</Button>
				<Button onClick={createProject}
								disabled={state == "creating" || checking || projectNameState == "err"}>{tc("projects:button:create")}</Button>
			</div>
		</DialogFooter>
		{dialog}
	</Dialog>;
}
