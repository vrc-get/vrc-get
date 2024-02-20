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
	Navbar,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React from "react";
import {
	ArrowPathIcon,
	ChevronDownIcon,
	EllipsisHorizontalIcon,
	GlobeAltIcon,
	MagnifyingGlassIcon,
	UserCircleIcon
} from "@heroicons/react/24/solid";

export default function Page() {
	const TABLE_HEAD = [
		"Name",
		"Type",
		"Unity",
		"Last Modified",
		"", // actions
	];

	// TODO: get data from backend and replace it
	const TABLE_DATA = [
		{name: "Test Project", path: "Path/to/Test Project", type: "Worlds", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
		{name: "Test Project", path: "Path/to/Test Project", type: "Avatars", unity: "2019.4.31f1", lastModified: "now"},
	]

	const cellClass = "p-2.5";

	return (
		<div className="m-4 flex flex-col overflow-hidden w-full gap-3">
			<ProjectViewHeader className={"flex-shrink-0"}/>
			<main className="flex-shrink overflow-hidden flex">
				<Card className="w-full overflow-x-auto overflow-y-scroll">
					<table className="relative table-auto text-left">
						<thead>
						<tr>
							{TABLE_HEAD.map((head, index) => (
								<th key={index}
										className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 ${cellClass}`}>
									<Typography variant="small" className="font-normal leading-none">{head}</Typography>
								</th>
							))}
						</tr>
						</thead>
						<tbody>
						{TABLE_DATA.map((row, index) => {
							const noGrowCellClass = `${cellClass} w-1`;
							const typeIconClass = `w-5 h-5`;
							return (
								<tr key={index} className="even:bg-blue-gray-50/50">
									<td className={cellClass}>
										<div className="flex flex-col">
											<Typography className="font-normal">
												{row.name}
											</Typography>
											<Typography className="font-normal opacity-50 text-sm">
												{row.path}
											</Typography>
										</div>
									</td>
									<td className={`${cellClass} w-[5em]`}>
										<div className="flex flex-col">
											<div className="flex justify-center items-center">
												<Typography className="font-normal">
													{row.type}
												</Typography>
											</div>
											<div className="flex justify-center items-center">
												{row.type === "Avatars" ? <UserCircleIcon className={typeIconClass}/> :
													<GlobeAltIcon className={typeIconClass}/>}
											</div>
										</div>
									</td>
									<td className={noGrowCellClass}>
										<Typography className="font-normal">
											{row.unity}
										</Typography>
									</td>
									<td className={noGrowCellClass}>
										<Typography className="font-normal">
											{row.lastModified}
										</Typography>
									</td>
									<td className={noGrowCellClass}>
										<div className="flex flex-row gap-2 max-w-min">
											<Button>Open Unity</Button>
											<Button color={"blue"}>Manage</Button>
											<Button color={"green"}>Backup</Button>
											<IconButton variant="text" color={"blue"}><EllipsisHorizontalIcon
												className={"size-5"}/></IconButton>
										</div>
									</td>
								</tr>
							)
						})}
						</tbody>
					</table>
				</Card>
			</main>
		</div>
	);
}

function ProjectViewHeader({className}: { className?: string }) {
	return (
		<Navbar className={`${className} mx-auto px-4 py-2`}>
			<div className="container mx-auto flex flex-wrap items-center justify-between text-blue-gray-900 gap-2">
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

				<CreateProjectButton
					onAddExistingProject={() => console.log("add existing")}
					onCreateNewProject={() => console.log("create new project")}/>
			</div>
		</Navbar>
	);
}

function CreateProjectButton(
	{onCreateNewProject, onAddExistingProject}: Readonly<{
		onCreateNewProject?: () => void,
		onAddExistingProject?: () => void
	}>,
) {
	const [opened, setOpened] = React.useState(false);

	const onClickMore = (e: React.MouseEvent<HTMLButtonElement>) => {
		e.stopPropagation()
		setOpened((prev) => !prev)
	};

	return (
		<Menu handler={(() => setOpened(false))} open={opened}>
			<MenuHandler>
				<ButtonGroup>
					<Button className={"pl-4 pr-3"} onClick={onCreateNewProject}>Create New Project</Button>
					<Button className={"pl-2 pr-2"} onClick={onClickMore}>
						<ChevronDownIcon className={"w-4 h-4"}/>
					</Button>
				</ButtonGroup>
			</MenuHandler>
			<MenuList>
				<MenuItem onClick={onAddExistingProject}>Add Existing Project</MenuItem>
			</MenuList>
		</Menu>
	);
}
