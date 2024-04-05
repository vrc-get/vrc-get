"use client"

import {
	Button,
	ButtonGroup,
	Card, Checkbox,
	Dialog,
	DialogBody,
	DialogFooter,
	DialogHeader,
	IconButton,
	Input,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Spinner,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React, {forwardRef, Fragment, useEffect, useMemo, useState} from "react";
import {
	ArrowPathIcon,
	ChevronDownIcon, ChevronUpDownIcon,
	EllipsisHorizontalIcon,
	GlobeAltIcon,
	QuestionMarkCircleIcon, StarIcon,
	UserCircleIcon
} from "@heroicons/react/24/solid";
import {HNavBar, VStack} from "@/components/layout";
import {
	environmentAddProjectWithPicker,
	environmentCheckProjectName,
	environmentCopyProjectForMigration,
	environmentCreateProject,
	environmentPickProjectDefaultPath,
	environmentProjectCreationInformation,
	environmentProjects, environmentSetFavoriteProject,
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
import {unsupported} from "@/lib/unsupported";
import {openUnity} from "@/lib/open-unity";
import {nop} from "@/lib/nop";
import {useDebounce} from "@uidotdev/usehooks";
import {VGOption, VGSelect} from "@/components/select";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {useRemoveProjectModal} from "@/lib/remove-project";
import {tc, tt} from "@/lib/i18n";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {pathSeparator} from "@/lib/os";
import {ChevronUpIcon} from "@heroicons/react/24/outline";

type SimpleSorting = "lastModified" | "name";
type Sorting = SimpleSorting | `${SimpleSorting}Reversed`;

export default function Page() {
	const result = useQuery({
		queryKey: ["projects"],
		queryFn: environmentProjects,
	});

	const [search, setSearch] = useState("");
	const [loadingOther, setLoadingOther] = useState(false);
	const [createProjectState, setCreateProjectState] = useState<'normal' | 'creating'>('normal');

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
				<Card className="w-full overflow-x-auto overflow-y-scroll">
					{
						result.status == "pending" ? tc("loading...") :
							result.status == "error" ? tc("error loading projects: {{msg}}", {msg: result.error.message}) :
								<ProjectsTable
									projects={result.data}
									search={search}
									loading={loading}
									refresh={() => result.refetch()}
									onRemoved={() => result.refetch()}
								/>
					}
				</Card>
				{createProjectState === "creating" &&
					<CreateProject close={() => setCreateProjectState("normal")} refetch={() => result.refetch()}/>}
			</main>
		</VStack>
	);
}

function ProjectsTable(
	{
		projects, search, onRemoved, loading, refresh,
	}: {
		projects: TauriProject[],
		search?: string,
		loading?: boolean,
		onRemoved?: () => void;
		refresh?: () => void,
	}
) {
	const [sorting, setSortingState] = useState<Sorting>("lastModified");

	const projectsShown = useMemo(() => {
		let searched = projects.filter(project => project.name.toLowerCase().includes(search?.toLowerCase() ?? ""));
		switch (sorting) {
			case "lastModified":
				searched.sort((a, b) => b.last_modified - a.last_modified);
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

	const thClass = `sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`;

	const setSorting = (simpleSorting: SimpleSorting) => {
		if (sorting === simpleSorting) {
			setSortingState(`${simpleSorting}Reversed`);
		} else if (sorting === `${simpleSorting}Reversed`) {
			setSortingState(simpleSorting);
		} else {
			setSortingState(simpleSorting);
		}
	}

	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				<th className={thClass}>
					<StarIcon className={"size-4"}/>
				</th>
				<th className={thClass}>
					<button className={"flex w-full"} onClick={() => setSorting("name")}>
						{
							sorting === "name" ? <ChevronDownIcon className={"size-3"}/>
								: sorting === "nameReversed" ? <ChevronUpIcon className={"size-3"}/>
									: <ChevronUpDownIcon className={"size-3"}/>
						}
						<Typography variant="small" className="font-normal leading-none">{tc("name")}</Typography>
					</button>
				</th>
				<th className={thClass}>
					<Typography variant="small" className="font-normal leading-none">{tc("type")}</Typography>
				</th>
				<th className={thClass}>
					<Typography variant="small" className="font-normal leading-none">{tc("unity")}</Typography>
				</th>
				<th className={thClass}>
					<button className={"flex w-full"} onClick={() => setSorting("lastModified")}>
						{
							sorting === "lastModified" ? <ChevronDownIcon className={"size-3"}/>
								: sorting === "lastModifiedReversed" ? <ChevronUpIcon className={"size-3"}/>
									: <ChevronUpDownIcon className={"size-3"}/>
						}
						<Typography variant="small" className="font-normal leading-none">{tc("last modified")}</Typography>
					</button>
				</th>
				<th className={thClass}></th>
			</tr>
			</thead>
			<tbody>
			{projectsShown.map((project) =>
				<ProjectRow key={project.index} project={project} loading={loading} refresh={refresh} onRemoved={onRemoved}/>)}
			</tbody>
		</table>
	);
}

const ProjectDisplayType: Record<TauriProjectType, "avatars" | "worlds" | "unknown"> = {
	"Unknown": "unknown",
	"LegacySdk2": "unknown",
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

const relativeTimeFormat = new Intl.RelativeTimeFormat("en", {style: 'short'});

function formatDateOffset(date: number) {
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

	if (diffAbs < 1000) return "just now";
	if (diffAbs < PER_MINUTE) return relativeTimeFormat.format(Math.floor(diff / PER_SECOND), "second");
	if (diffAbs < PER_HOUR) return relativeTimeFormat.format(Math.floor(diff / PER_MINUTE), "minute");
	if (diffAbs < PER_DAY) return relativeTimeFormat.format(Math.floor(diff / PER_HOUR), "hour");
	if (diffAbs < PER_WEEK) return relativeTimeFormat.format(Math.floor(diff / PER_DAY), "day");
	if (diffAbs < PER_MONTH) return relativeTimeFormat.format(Math.floor(diff / PER_WEEK), "week");
	if (diffAbs < PER_YEAR) return relativeTimeFormat.format(Math.floor(diff / PER_MONTH), "month");

	return relativeTimeFormat.format(Math.floor(diff / PER_YEAR), "year");
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
		onRemoved,
		loading,
		refresh,
	}: {
		project: TauriProject;
		onRemoved?: () => void;
		loading?: boolean;
		refresh?: () => void;
	}
) {
	const router = useRouter();

	const [dialogStatus, setDialogStatus] = useState<ProjectRowState>({type: 'normal'});
	const removeProjectModal = useRemoveProjectModal({onRemoved});

	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const typeIconClass = `w-5 h-5`;

	const projectTypeKind = ProjectDisplayType[project.project_type] ?? "unknown";
	const displayType = tc(projectTypeKind)
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
			toastSuccess(tt("project migrated successfully"));
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
			return <Tooltip content={tt("project folder does not exist")}>
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
				<Tooltip content={"Legacy SDK2 project cannot be migrated automatically. Please migrate to SDK3 first."}>
					<RowButton color={"light-green"} disabled>
						{tc("migrate")}
					</RowButton>
				</Tooltip>
			break;
		case "LegacyWorlds":
		case "LegacyAvatars":
			manageButton = <RowButton color={"light-green"} onClick={startMigrateVpm}>{tc("migrate")}</RowButton>
			break;
		case "UpmWorlds":
		case "UpmAvatars":
		case "UpmStarter":
			manageButton = <Tooltip content={"UPM-VCC projects are not supported"}>
				<RowButton color={"blue"} disabled>
					{tc("manage")}
				</RowButton>
			</Tooltip>
			break;
		case "Unknown":
		case "Worlds":
		case "Avatars":
		case "VpmStarter":
			manageButton = <RowButton
				onClick={() => router.push(`/projects/manage?${new URLSearchParams({projectPath: project.path})}`)}
				color={"blue"}>
				{tc("manage")}
			</RowButton>
			break;
	}

	let dialogContent: React.ReactNode = null;
	switch (dialogStatus.type) {
		case "migrateVpm:confirm":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("vpm migration")}</DialogHeader>
					<DialogBody>
						<Typography className={"text-red-700"}>
							{tc("project migration is experimental in vrc-get.")}
						</Typography>
						<Typography className={"text-red-700"}>
							{tc("please make backup of your project before migration.")}
						</Typography>
					</DialogBody>
					<DialogFooter>
						<Button onClick={() => setDialogStatus({type: "normal"})} className="mr-1">{tc("cancel")}</Button>
						<Button onClick={() => doMigrateVpm(false)} color={"red"} className="mr-1">{tc("migrate a copy")}</Button>
						<Button onClick={() => doMigrateVpm(true)} color={"red"}>{tc("migrate in-place")}</Button>
					</DialogFooter>
				</Dialog>
			);
			break;
		case "migrateVpm:copyingProject":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("vpm migration")}</DialogHeader>
					<DialogBody>
						<Typography>
							{tc("copying project for migration...")}
						</Typography>
					</DialogBody>
				</Dialog>
			);
			break;
		case "migrateVpm:updating":
			dialogContent = (
				<Dialog open handler={nop} className={"whitespace-normal"}>
					<DialogHeader>{tc("vpm migration")}</DialogHeader>
					<DialogBody>
						<Typography>
							{tc("migrating project...")}
						</Typography>
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
									className="hover:before:content-none"/>
			</td>
			<td className={cellClass}>
				<MayTooltip content={tc("project folder does not exist")}>
					<div className="flex flex-col">
						<Typography className="font-normal whitespace-pre">
							{project.name}
						</Typography>
						<Typography className="font-normal opacity-50 text-sm whitespace-pre">
							{project.path}
						</Typography>
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
						<Typography className="font-normal">
							{displayType}
						</Typography>
						{isLegacy &&
							<Typography className="font-normal opacity-50 text-sm text-red-700">{tc("legacy")}</Typography>}
					</div>
				</div>
			</td>
			<td className={noGrowCellClass}>
				<Typography className="font-normal">
					{project.unity}
				</Typography>
			</td>
			<td className={noGrowCellClass}>
				<Tooltip content={lastModifiedHumanReadable}>
					<time dateTime={lastModified.toISOString()}>
						<Typography as={"time"} className="font-normal">
							{formatDateOffset(project.last_modified)}
						</Typography>
					</time>
				</Tooltip>
			</td>
			<td className={noGrowCellClass}>
				<div className="flex flex-row gap-2 max-w-min">
					<RowButton onClick={() => openUnity(project.path)}>{tc("open unity")}</RowButton>
					{manageButton}
					<RowButton onClick={unsupported("Backup")} color={"green"}>{tc("backup")}</RowButton>
					<Menu>
						<MenuHandler>
							<IconButton variant="text" color={"blue"}><EllipsisHorizontalIcon
								className={"size-5"}/></IconButton>
						</MenuHandler>
						<MenuList>
							<MenuItem onClick={openProjectFolder}
												disabled={removed || loading}>{tc("open project folder")}</MenuItem>
							<MenuItem onClick={() => removeProjectModal.startRemove(project)} disabled={loading}
												className={'text-red-700 focus:text-red-700'}>
								{tc("remove project")}
							</MenuItem>
						</MenuList>
					</Menu>
				</div>
				{dialogContent}
				{removeProjectModal.dialog}
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
					toastError(tt("invalid folder is selected"));
					break;
				case "Successful":
					toastSuccess(tt("added project successfully"));
					refresh?.();
					break;
				case "AlreadyAdded":
					toastError(tt("the project is already added"));
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
			<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
				{tc("projects")}
			</Typography>

			<Tooltip content="Reflesh list of projects">
				<IconButton variant={"text"} onClick={() => refresh?.()} disabled={isLoading}>
					{isLoading ? <Spinner className="w-5 h-5"/> : <ArrowPathIcon className={"w-5 h-5"}/>}
				</IconButton>
			</Tooltip>

			<SearchBox className={"w-max flex-grow"} value={search} onChange={(e) => setSearch(e.target.value)}/>

			<Menu>
				<ButtonGroup>
					<Button className={"pl-4 pr-3"} onClick={startCreateProject}>{tc("create new project")}</Button>
					<MenuHandler className={"pl-2 pr-2"}>
						<Button>
							<ChevronDownIcon className={"w-4 h-4"}/>
						</Button>
					</MenuHandler>
				</ButtonGroup>
				<MenuList>
					<MenuItem onClick={addProject}>{tc("add existing project")}</MenuItem>
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
	const [state, setState] = useState<CreateProjectstate>('loadingInitialInformation');
	const [projectNameCheckState, setProjectNameCheckState] = useState<'checking' | TauriProjectDirCheckResult>('Ok');

	const [templates, setTemplates] = useState<TauriProjectTemplate[]>([]);
	const [chosenTemplate, setChosenTemplate] = useState<TauriProjectTemplate>();
	const [projectNameRaw, setProjectName] = useState("New Project");
	const projectName = projectNameRaw.trim();
	const [projectLocation, setProjectLocation] = useState("");
	const projectNameDebounced = useDebounce(projectName, 500);

	const [pickProjectDefaultPath, dialog] = useFilePickerFunction(environmentPickProjectDefaultPath);

	useEffect(() => {
		(async () => {
			const information = await environmentProjectCreationInformation();
			setTemplates(information.templates);
			setChosenTemplate(information.templates[0]);
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
					toastError(tt("the selected directory is invalid"));
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
			await environmentCreateProject(projectLocation, projectName, chosenTemplate!);
			toastSuccess(tt("project created successfully"));
			close?.();
			refetch?.();
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
			projectNameCheck = tc("ready to create a project");
			projectNameState = "Ok";
			break;
		case "InvalidNameForFolderName":
			projectNameCheck = tc("invalid project name");
			projectNameState = "err";
			break;
		case "MayCompatibilityProblem":
			projectNameCheck = tc("using such a symbol may cause problems");
			projectNameState = "warn";
			break;
		case "WideChar":
			projectNameCheck = tc("using multibyte characters may cause problems");
			projectNameState = "warn";
			break;
		case "AlreadyExists":
			projectNameCheck = tc("the directory already exists");
			projectNameState = "err";
			break;
		case "checking":
			projectNameCheck = <><Spinner/> {tc("checking the directory name...")}</>;
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
			dialogBody = <>
				<VStack>
					<div className={"flex gap-1"}>
						<div className={"flex items-center"}>
							<Typography as={"label"}>{tc("template:")}</Typography>
						</div>
						<VGSelect menuClassName={"z-[19999]"} value={chosenTemplate?.name}
											onChange={value => setChosenTemplate(value)}>
							{templates.map(template =>
								<VGOption value={template} key={`${template.type}:${template.name}`}>{template.name}</VGOption>)}
						</VGSelect>
					</div>
					<Input label={"Project Name"} value={projectNameRaw} onChange={(e) => setProjectName(e.target.value)}/>
					<div className={"flex gap-1"}>
						<Input label={"Project Location"} value={projectLocation} disabled/>
						<Button className={"px-4"} onClick={selectProjectDefaultFolder}>{tc("select directory")}</Button>
					</div>
					<Typography variant={"small"} className={"whitespace-normal"}>
						{tc("the new project will be at <code>{{path}}</code>", {path: `${projectLocation}${pathSeparator()}${projectName}`}, {
							components: {
								code: <code className={"whitespace-pre"}/>
							}
						})}
					</Typography>
					<Typography variant={"small"} className={`whitespace-normal ${projectNameStateClass}`}>
						{projectNameCheck}
					</Typography>
				</VStack>
			</>;
			break;
		case "creating":
			dialogBody = <>
				<Spinner/>
				<Typography>{tc("creating the project...")}</Typography>
			</>;
			break;
	}

	return <Dialog handler={nop} open>
		<DialogHeader>{tc("create new project")}</DialogHeader>
		<DialogBody>
			{dialogBody}
		</DialogBody>
		<DialogFooter>
			<div className={"flex gap-2"}>
				<Button onClick={close} disabled={state == "creating"}>{tc("cancel")}</Button>
				<Button onClick={createProject}
								disabled={state == "creating" || checking || projectNameState == "err"}>{tc("create")}</Button>
			</div>
		</DialogFooter>
		{dialog}
	</Dialog>;
}
