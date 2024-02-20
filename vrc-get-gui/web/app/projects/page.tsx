"use client"

import {
	Button,
	ButtonGroup,
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
import {ArrowPathIcon, ChevronDownIcon, MagnifyingGlassIcon} from "@heroicons/react/24/solid";

export default function Page() {
	return (
		<div className="p-4">
			<ProjectViewHeader/>
			<main className="flex flex-col items-center justify-between">
				TODO: list of projects
			</main>
		</div>
	);
}

function ProjectViewHeader() {
	return (
		<Navbar className="mx-auto px-4 py-2">
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
