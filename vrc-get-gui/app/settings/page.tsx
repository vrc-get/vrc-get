"use client"

import {Button, Card, Input, Typography} from "@material-tailwind/react";
import Link from "next/link";
import {useQuery} from "@tanstack/react-query";
import {
	environmentGetSettings,
	environmentPickProjectBackupPath,
	environmentPickProjectDefaultPath,
	environmentPickUnity,
	environmentPickUnityHub,
	TauriEnvironmentSettings
} from "@/lib/bindings";
import {VStack} from "@/components/layout";
import {toastThrownError} from "@/lib/toastThrownError";
import React from "react";
import {toast} from "react-toastify";

export default function Page() {
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	let body;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>Error loading Settings</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>Loading...</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch}/>;
			break;
		default:
			const _exhaustiveCheck: never = result;
	}

	return (
		<VStack className={"p-4 overflow-y-auto"}>
			{body}
			<Card className={"flex-shrink-0 p-4"}>
				<h2>Licenses</h2>
				<Typography className={"whitespace-normal"}>
					Click <Link href={"/settings/licenses"} className={"underline"}>here</Link> to view licenses of the projects
					used in vrc-get-gui.
				</Typography>
			</Card>
		</VStack>
	);
}

function Settings(
	{
		settings,
		refetch,
	}: {
		settings: TauriEnvironmentSettings,
		refetch: () => void
	}
) {
	const selectUnityHub = async () => {
		try {
			const result = await environmentPickUnityHub();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toast.error("Selected file is invalid as a Unity Hub");
					break;
				case "Successful":
					toast.success("Updated Unity Hub successfully!");
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const addUnity = async () => {
		try {
			const result = await environmentPickUnity();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toast.error("Selected file is invalid as a Unity");
					break;
				case "AlreadyAdded":
					toast.error("Selected unity is already added");
					break;
				case "Successful":
					toast.success("Updated Unity Hub successfully!");
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const selectProjectDefaultFolder = async () => {
		try {
			const result = await environmentPickProjectDefaultPath();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toast.error("Selected file is invalid as a Project Default Path");
					break;
				case "Successful":
					toast.success("Updated Project Default Path successfully!");
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	};

	const selectProjectBackupFolder = async () => {
		try {
			const result = await environmentPickProjectBackupPath();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toast.error("Selected file is invalid as a Project Backup Path");
					break;
				case "Successful":
					toast.success("Updated Project Backup Path successfully!");
					refetch()
					break;
				default:
					const _exhaustiveCheck: never = result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	};

	return (
		<>
			<Card className={"flex-shrink-0 p-4"}>
				<h1>Settings</h1>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2 className={"pb-2"}>Unity Hub</h2>
				<div className={"flex gap-1"}>
					{
						settings.unity_hub
							? <Input value={settings.unity_hub} disabled/>
							: <Input value={"Unity Hub Not Found"} disabled className={"text-red-900"}/>
					}
					<Button className={"px-4"} onClick={selectUnityHub}>Select</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<div className={"pb-2 flex align-middle"}>
					<div className={"flex-grow flex items-center"}>
						<h2>Unity Installation</h2>
					</div>
					<Button onClick={addUnity} size={"sm"} className={"m-1"}>Add Unity</Button>
				</div>
				<Card className="w-full overflow-x-auto overflow-y-scroll min-h-[20vh]">
					<UnityTable unityPaths={settings.unity_paths}/>
				</Card>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>Default Project Path</h2>
				<Typography className={"whitespace-normal"}>
					The Default Project Path is the directory where vrc-get-gui will create new projects in.
				</Typography>
				<div className={"flex gap-1"}>
					<Input value={settings.default_project_path} disabled/>
					<Button className={"px-4"} onClick={selectProjectDefaultFolder}>Select Folder</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>Backup Path</h2>
				<Typography className={"whitespace-normal"}>
					The Backup Path is the directory where vrc-get-gui will create backups zip of projects.
				</Typography>
				<div className={"flex gap-1"}>
					<Input value={settings.project_backup_path} disabled/>
					<Button className={"px-4"} onClick={selectProjectBackupFolder}>Select Folder</Button>
				</div>
			</Card>
		</>
	)
}

function UnityTable(
	{
		unityPaths,
	}: {
		unityPaths: [path: string, version: string, fromHub: boolean][]
	}
) {
	const UNITY_TABLE_HEAD = ["Unity Version", "Unity Path", "Source"];
	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				{UNITY_TABLE_HEAD.map((head, index) => (
					<th key={index}
							className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
						<Typography variant="small" className="font-normal leading-none">{head}</Typography>
					</th>
				))}
			</tr>
			</thead>
			<tbody>
			{
				unityPaths.map(([path, version, isFromHub]) => (
					<tr key={path}>
						<td className={"p-2.5"}>{version}</td>
						<td className={"p-2.5"}>{path}</td>
						<td className={"p-2.5"}>
							{isFromHub ? "Unity Hub" : "Manual"}
						</td>
					</tr>
				))
			}
			</tbody>
		</table>
	)
}
