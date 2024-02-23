"use client"

import {
	Button,
	ButtonGroup,
	Card,
	IconButton,
	Menu,
	MenuHandler,
	MenuItem,
	MenuList,
	Navbar,
	Option,
	Select,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import React from "react";
import {ArrowLeftIcon, ChevronDownIcon,} from "@heroicons/react/24/solid";
import {MinusCircleIcon, PlusCircleIcon,} from "@heroicons/react/24/outline";

export default function Page() {
	const TABLE_HEAD = [
		"Package",
		"Installed",
		"Latest",
		"Source",
		"", // actions
	];

	// TODO: get data from backend and replace it
	const TABLE_DATA = [
		{
			displayName: "Avatar Optimizer",
			id: "com.anatawa12.avatar-optimizer",
			versions: [
				"0.0.1",
				"0.0.2",
				"0.1.0",
				"0.1.2",
				"0.1.3",
				"0.1.4",
				"0.2.0",
				"0.2.0-beta.1",
				"0.2.0-beta.2",
				"0.2.0-rc.1",
				"0.2.0-rc.2",
				"0.2.1",
				"0.2.1-beta.1",
				"0.2.2",
				"0.2.3",
				"0.2.4",
				"0.2.5",
				"0.2.5-rc.1",
				"0.2.6",
				"0.2.6-rc.1",
				"0.2.6-rc.2",
				"0.2.6-rc.3",
				"0.2.6-rc.4",
				"0.2.7",
				"0.2.7-beta.1",
				"0.2.8",
				"0.2.8-rc.1",
				"0.3.0",
				"0.3.0-beta.1",
				"0.3.0-beta.2",
				"0.3.0-beta.3",
				"0.3.0-rc.1",
				"0.3.0-rc.2",
				"0.3.1",
				"0.3.2",
				"0.3.2-beta.1",
				"0.3.2-beta.2",
				"0.3.3",
				"0.3.4",
				"0.3.5",
				"0.4.0",
				"0.4.0-beta.1",
				"0.4.0-rc.1",
				"0.4.0-rc.2",
				"0.4.1",
				"0.4.1-beta.1",
				"0.4.1-rc.1",
				"0.4.1-rc.2",
				"0.4.1-rc.3",
				"0.4.10",
				"0.4.10-beta.1",
				"0.4.11",
				"0.4.12",
				"0.4.2",
				"0.4.3",
				"0.4.4",
				"0.4.4-rc.1",
				"0.4.5",
				"0.4.5-beta.1",
				"0.4.6",
				"0.4.7",
				"0.4.8",
				"0.4.9",
				"1.0.0",
				"1.0.0-beta.1",
				"1.0.0-beta.2",
				"1.0.0-beta.3",
				"1.0.0-beta.4",
				"1.0.0-beta.5",
				"1.1.0",
				"1.1.0-beta.1",
				"1.1.0-beta.2",
				"1.1.0-rc.1",
				"1.1.1",
				"1.1.2-beta.1",
				"1.2.0",
				"1.2.0-beta.1",
				"1.2.0-rc.1",
				"1.3.0",
				"1.3.0-rc.1",
				"1.3.0-rc.2",
				"1.3.1",
				"1.3.2",
				"1.3.2-beta.1",
				"1.3.2-beta.2",
				"1.3.2-beta.3",
				"1.3.3",
				"1.3.4",
				"1.4.0",
				"1.4.0-beta.1",
				"1.4.0-rc.1",
				"1.4.0-rc.2",
				"1.4.0-rc.3",
				"1.4.0-rc.4",
				"1.4.1",
				"1.4.2",
				"1.4.3",
				"1.4.3-beta.1",
				"1.5.0",
				"1.5.0-beta.1",
				"1.5.0-beta.10",
				"1.5.0-beta.11",
				"1.5.0-beta.12",
				"1.5.0-beta.13",
				"1.5.0-beta.14",
				"1.5.0-beta.2",
				"1.5.0-beta.3",
				"1.5.0-beta.4",
				"1.5.0-beta.5",
				"1.5.0-beta.6",
				"1.5.0-beta.7",
				"1.5.0-beta.8",
				"1.5.0-beta.9",
				"1.5.0-rc.1",
				"1.5.0-rc.10",
				"1.5.0-rc.11",
				"1.5.0-rc.12",
				"1.5.0-rc.13",
				"1.5.0-rc.2",
				"1.5.0-rc.3",
				"1.5.0-rc.4",
				"1.5.0-rc.5",
				"1.5.0-rc.6",
				"1.5.0-rc.7",
				"1.5.0-rc.8",
				"1.5.0-rc.9",
				"1.5.1",
				"1.5.1-beta.1",
				"1.5.10",
				"1.5.11",
				"1.5.11-beta.1",
				"1.5.2",
				"1.5.2-beta.1",
				"1.5.2-beta.2",
				"1.5.2-beta.3",
				"1.5.3",
				"1.5.3-beta.1",
				"1.5.4",
				"1.5.5",
				"1.5.5-beta.1",
				"1.5.5-rc.1",
				"1.5.6",
				"1.5.6-beta.1",
				"1.5.6-beta.2",
				"1.5.6-rc.1",
				"1.5.7",
				"1.5.7-beta.1",
				"1.5.8",
				"1.5.8-rc.1",
				"1.5.9",
				"1.5.9-rc.1",
				"1.6.0",
				"1.6.0-beta.1",
				"1.6.0-beta.10",
				"1.6.0-beta.11",
				"1.6.0-beta.12",
				"1.6.0-beta.2",
				"1.6.0-beta.3",
				"1.6.0-beta.4",
				"1.6.0-beta.5",
				"1.6.0-beta.6",
				"1.6.0-beta.7",
				"1.6.0-beta.8",
				"1.6.0-beta.9",
				"1.6.0-rc.1",
				"1.6.0-rc.2",
				"1.6.0-rc.3",
				"1.6.0-rc.4",
				"1.6.1",
				"1.6.2",
				"1.6.2-rc.1",
				"1.6.3",
				"1.6.4",
				"1.6.4-beta.1",
				"1.6.5",
				"1.6.5-beta.1",
				"1.6.5-rc.1",
				"1.6.5-rc.2",
				"1.6.5-rc.3",
				"1.6.6",
			],
			installed: "1.0.0",
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
		{
			displayName: "NDM Framework",
			id: "dev.nadena.ndmf",
			versions: [
				"1.0.0",
				"1.0.1",
				"1.0.2",
				"1.1.0",
				"1.1.1",
				"1.2.0",
				"1.2.1",
				"1.2.2",
				"1.2.3",
				"1.2.4",
				"1.2.5",
				"1.3.0",
				"1.3.1",
				"1.3.2",
				"1.3.3",
				"1.3.4"
			],
			installed: null,
			source: "anatawa12",
		},
	]

	const unityVersions = [
		'2019.4.31f1',
		'2020.3.14f1',
		'2021.1.5f1',
	];

	const cellClass = "p-2.5";

	return (
		<div className="m-4 flex flex-col overflow-hidden w-full gap-3">
			<ProjectViewHeader className={"flex-shrink-0"}/>
			<Card className={"flex-shrink-0 p-2 flex flex-row"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
					located at: <code className={"bg-gray-200 p-0.5"}>/path/to/project</code>
				</Typography>
				<div className={"flex-grow flex-shrink"}></div>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
					Unity Version:
				</Typography>
				<div className={"flex-grow-0 flex-shrink-0"}>
					<Select variant={'outlined'} value={"2019.4.31f1"} labelProps={{className: "hidden"}}
									className="border-blue-gray-200">
						{unityVersions.map(v => <Option value={v}>{v}</Option>)}
					</Select>
				</div>
			</Card>
			<main className="flex-shrink overflow-hidden flex">
				<Card className="w-full p-2 flex-grow flex-shrink flex">
					<div className={"flex flex-shrink-0 flex-grow-0 flex-row"}>
						<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0 flex-shrink-0">
							Manage Packages
						</Typography>
					</div>
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
								return (
									<tr key={index} className="even:bg-blue-gray-50/50">
										<td className={cellClass}>
											<div className="flex flex-col">
												<Typography className="font-normal">
													{row.displayName}
												</Typography>
												<Typography className="font-normal opacity-50 text-sm">
													{row.id}
												</Typography>
											</div>
										</td>
										<td className={noGrowCellClass}>
											{/* This is broken: popup is not shown out of the card */}
											<Select value={row.installed ?? "Not Installed"} labelProps={{className: "hidden"}}
															menuProps={{className: "z-20"}} className="border-blue-gray-200">
												{row.versions.map(v => <Option value={v}>{v}</Option>)}
											</Select>
										</td>
										<td className={noGrowCellClass}>
											<Typography className="font-normal">
												{row.versions[row.versions.length - 1]}
											</Typography>
										</td>
										<td className={noGrowCellClass}>
											<Typography className="font-normal">
												{row.source}
											</Typography>
										</td>
										<td className={noGrowCellClass}>
											<div className="flex flex-row gap-2 max-w-min">
												{
													row.installed ? (
														<Tooltip content={"Remove Package"}>
															<IconButton variant={'text'}><MinusCircleIcon
																className={"size-5 text-red-700"}/></IconButton>
														</Tooltip>
													) : (
														<Tooltip content={"Add Package"}>
															<IconButton variant={'text'}><PlusCircleIcon
																className={"size-5 text-gray-800"}/></IconButton>
														</Tooltip>
													)
												}
											</div>
										</td>
									</tr>
								)
							})}
							</tbody>
						</table>
					</Card>
				</Card>
			</main>
		</div>
	);
}

function ProjectViewHeader({className}: { className?: string }) {
	return (
		<Navbar className={`${className} mx-auto px-4 py-2`}>
			<div className="container mx-auto flex flex-wrap items-center justify-between text-blue-gray-900 gap-2">
				<Tooltip content="Back to projects">
					<IconButton variant={"text"} onClick={() => history.back()}>
						<ArrowLeftIcon className={"w-5 h-5"}/>
					</IconButton>
				</Tooltip>

				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					Project Name
				</Typography>

				<div className="relative flex gap-2 w-max flex-grow">
				</div>

				<OpenProjectButton
					openUnity={() => console.log("create new project")}
				/>
			</div>
		</Navbar>
	);
}

function OpenProjectButton(
	{openUnity, openProjectFolder, makeBackup, removeProject}: Readonly<{
		openUnity?: () => void,
		openProjectFolder?: () => void,
		makeBackup?: () => void
		removeProject?: () => void
	}>,
) {
	return (
		<Menu>
			<ButtonGroup>
				<Button className={"pl-4 pr-3"} onClick={openUnity}>Open Unity</Button>
				<MenuHandler className={"pl-2 pr-2"}>
					<Button>
						<ChevronDownIcon className={"w-4 h-4"}/>
					</Button>
				</MenuHandler>
			</ButtonGroup>
			<MenuList>
				<MenuItem onClick={openProjectFolder}>Open Project Folder</MenuItem>
				<MenuItem onClick={makeBackup}>Make Backup</MenuItem>
				<MenuItem className={"bg-red-700 text-white"} onClick={removeProject}>Remove Project</MenuItem>
			</MenuList>
		</Menu>
	);
}
