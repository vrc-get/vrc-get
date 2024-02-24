"use client"

import {
	Button,
	ButtonGroup,
	Card,
	IconButton,
	Input,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React, {useEffect, useState} from "react";
import {
	ArrowPathIcon,
	ChevronDownIcon,
	EllipsisHorizontalIcon,
	GlobeAltIcon,
	MagnifyingGlassIcon,
	QuestionMarkCircleIcon,
	UserCircleIcon
} from "@heroicons/react/24/solid";
import {HNavBar, VStack} from "@/components/layout";
import {environmentProjects, TauriProject, TauriProjectType} from "@/lib/generated/bindings";

export default function Page() {
	const TABLE_HEAD = [
		"Name",
		"Type",
		"Unity",
		"Last Modified",
		"", // actions
	];

	const [projects, setProjects] = useState<TauriProject[]>([]);
	useEffect(() => {
		// TODO: loading animation and error handling
		environmentProjects().then(setProjects);
	});

	return (
		<VStack className={"m-4"}>
			<ProjectViewHeader className={"flex-shrink-0"}/>
			<main className="flex-shrink overflow-hidden flex">
				<Card className="w-full overflow-x-auto overflow-y-scroll">
					<table className="relative table-auto text-left">
						<thead>
						<tr>
							{TABLE_HEAD.map((head, index) => (
								<th key={index}
										className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
									<Typography variant="small" className="font-normal leading-none">{head}</Typography>
								</th>
							))}
						</tr>
						</thead>
						<tbody>
						{projects.map((project) => <ProjectRow key={project.path} project={project}/>)}
						</tbody>
					</table>
				</Card>
			</main>
		</VStack>
	);
}

const ProjectDisplayType: Record<TauriProjectType, "Avatars" | "Worlds" | "Unknown"> = {
	"Unknown": "Unknown",
	"LegacySdk2": "Unknown",
	"LegacyWorlds": "Worlds",
	"LegacyAvatars": "Avatars",
	"UpmWorlds": "Worlds",
	"UpmAvatars": "Avatars",
	"UpmStarter": "Unknown",
	"Worlds": "Worlds",
	"Avatars": "Avatars",
	"VpmStarter": "Unknown",
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

function ProjectRow({project}: { project: TauriProject }) {
	const cellClass = "p-2.5";
	const noGrowCellClass = `${cellClass} w-1`;
	const typeIconClass = `w-5 h-5`;

	const displayType = ProjectDisplayType[project.project_type] ?? "Unknown"
	const isLegacy = LegacyProjectTypes.includes(project.project_type);
	const lastModified = new Date(project.last_modified);
	const lastModifiedHumanReadable = `${lastModified.getFullYear().toString().padStart(4, '0')}-${(lastModified.getMonth() + 1).toString().padStart(2, '0')}-${lastModified.getDate().toString().padStart(2, '0')} ${lastModified.getHours().toString().padStart(2, "0")}:${lastModified.getMinutes().toString().padStart(2, "0")}:${lastModified.getSeconds().toString().padStart(2, "0")}`;

	return (
		<tr className="even:bg-blue-gray-50/50">
			<td className={cellClass}>
				<div className="flex flex-col">
					<Typography className="font-normal">
						{project.name}
					</Typography>
					<Typography className="font-normal opacity-50 text-sm">
						{project.path}
					</Typography>
				</div>
			</td>
			<td className={`${cellClass} w-[8em]`}>
				<div className="flex flex-row gap-2">
					<div className="flex items-center">
						{displayType === "Avatars" ? <UserCircleIcon className={typeIconClass}/> :
							displayType === "Worlds" ? <GlobeAltIcon className={typeIconClass}/> :
								<QuestionMarkCircleIcon className={typeIconClass}/>}
					</div>
					<div className="flex flex-col justify-center">
						<Typography className="font-normal">
							{displayType}
						</Typography>
						{isLegacy && <Typography className="font-normal opacity-50 text-sm text-red-700">Legacy</Typography>}
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
					<Button>Open Unity</Button>
					<Button onClick={() => location.href = "/projects/manage"} color={"blue"}>Manage</Button>
					<Button color={"green"}>Backup</Button>
					<IconButton variant="text" color={"blue"}><EllipsisHorizontalIcon
						className={"size-5"}/></IconButton>
				</div>
			</td>
		</tr>
	)
}

function ProjectViewHeader({className}: { className?: string }) {
	return (
		<HNavBar className={className}>
			<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
				Projects
			</Typography>

			<Tooltip content="Reflesh list of projects">
				<IconButton variant={"text"} onClick={() => console.log("click")}>
					<ArrowPathIcon className={"w-5 h-5"}/>
				</IconButton>
			</Tooltip>

			<div className="relative flex gap-2 w-max flex-grow">
				{/* The search box */}
				<Input
					type="search"
					placeholder="Search"
					containerProps={{
						className: "min-w-[100px]",
					}}
					className=" !border-t-blue-gray-300 pl-9 placeholder:text-blue-gray-300 focus:!border-blue-gray-300"
					labelProps={{
						className: "before:content-none after:content-none",
					}}
				/>
				<MagnifyingGlassIcon className="!absolute left-3 top-[13px]" width={13} height={14}/>
			</div>

			<Menu>
				<ButtonGroup>
					<Button className={"pl-4 pr-3"}>Create New Project</Button>
					<MenuHandler className={"pl-2 pr-2"}>
						<Button>
							<ChevronDownIcon className={"w-4 h-4"}/>
						</Button>
					</MenuHandler>
				</ButtonGroup>
				<MenuList>
					<MenuItem>Add Existing Project</MenuItem>
				</MenuList>
			</Menu>
		</HNavBar>
	);
}
