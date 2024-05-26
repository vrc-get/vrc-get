"use client"

import {Button} from "@/components/ui/button";
import {Card, CardHeader} from "@/components/ui/card";
import {Checkbox} from "@/components/ui/checkbox";
import {Input} from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import Link from "next/link";
import {useQuery} from "@tanstack/react-query";
import {
	deepLinkInstallVcc,
	environmentGetSettings,
	environmentPickProjectBackupPath,
	environmentPickProjectDefaultPath,
	environmentPickUnity,
	environmentPickUnityHub,
	environmentSetBackupFormat,
	environmentSetLanguage,
	environmentSetTheme,
	environmentSetShowPrereleasePackages,
	environmentLanguage,
	environmentTheme,
	TauriEnvironmentSettings,
	utilGetVersion,
} from "@/lib/bindings";
import {HNavBar, VStack} from "@/components/layout";
import React from "react";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import i18next, {languages, tc, tt} from "@/lib/i18n";
import {useFilePickerFunction} from "@/lib/use-file-picker-dialog";
import {emit} from "@tauri-apps/api/event";
import {shellOpen} from "@/lib/shellOpen";
import {loadOSApi} from "@/lib/os";
import type {OsType} from "@tauri-apps/api/os";

export default function Page() {
	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	let body;
	switch (result.status) {
		case "error":
			body = <Card className={"p-4"}>{tc("settings:error:load error")}</Card>;
			break;
		case "pending":
			body = <Card className={"p-4"}>{tc("general:loading...")}</Card>;
			break;
		case "success":
			body = <Settings settings={result.data} refetch={result.refetch}/>;
			break;
		default:
			const _exhaustiveCheck: never = result;
	}

	return (
		<VStack className={"p-4"}>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("settings")}
				</p>
			</HNavBar>
			{body}
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
	const [pickUnity, unityDialog] = useFilePickerFunction(environmentPickUnity);
	const [pickUnityHub, unityHubDialog] = useFilePickerFunction(environmentPickUnityHub);
	const [pickProjectDefaultPath, projectDefaultDialog] = useFilePickerFunction(environmentPickProjectDefaultPath);
	const [pickProjectBackupPath, projectBackupDialog] = useFilePickerFunction(environmentPickProjectBackupPath);

	const [osType, setOsType] = React.useState<OsType>("Windows_NT");

	React.useEffect(() => {
		(async () => {
			const os = await loadOSApi();
			setOsType(await os.type());
		})();
	}, [])

	const selectUnityHub = async () => {
		try {
			const result = await pickUnityHub();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("settings:toast:not unity hub"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:unity hub path updated"));
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
			const result = await pickUnity();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("settings:toast:not unity"));
					break;
				case "AlreadyAdded":
					toastError(tt("settings:toast:unity already added"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:unity added"));
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
			const result = await pickProjectDefaultPath();
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:default project path updated"));
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
			const result = await pickProjectBackupPath();
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("settings:toast:backup path updated"));
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

	const setBackupFormat = async (format: string) => {
		try {
			await environmentSetBackupFormat(format)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const toggleShowPrereleasePackages = async (e: "indeterminate" | boolean) => {
		try {
			await environmentSetShowPrereleasePackages(e===true)
			refetch()
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}

	const {data: lang, refetch: refetchLang} = useQuery({
		queryKey: ["environmentLanguage"],
		queryFn: environmentLanguage
	})

	const changeLanguage = async (value: string) => {
		await Promise.all([
			i18next.changeLanguage(value),
			environmentSetLanguage(value),
      refetchLang(),
		])
	};

	const [theme, setTheme] = React.useState<string | null>(null);

	React.useEffect(() => {
		(async () => {
			const theme = await environmentTheme();
			setTheme(theme);
		})();
	}, [])

	const changeTheme = async (theme: string) => {
		await environmentSetTheme(theme);
		setTheme(theme);
		if (theme === "system") {
			const {appWindow} = await import("@tauri-apps/api/window");
			theme = await appWindow.theme() ?? "light";
		}
		document.documentElement.setAttribute("class", theme);
	};

	const reportIssue = async () => {
		const url = new URL("https://github.com/vrc-get/vrc-get/issues/new")
		url.searchParams.append("labels", "bug,vrc-get-gui")
		url.searchParams.append("template", "01_gui_bug-report.yml")
		const osApi = await loadOSApi();
		url.searchParams.append("os", `${await osApi.type()} - ${await osApi.platform()} - ${await osApi.version()} - ${await osApi.arch()}`)
		const appVersion = await utilGetVersion();
		url.searchParams.append("version", appVersion)
		url.searchParams.append("version", appVersion)

		shellOpen(url.toString())
	}

	const installVccProtocol = async () => {
		try {
			await deepLinkInstallVcc();
			toastSuccess(tc("settings:toast:vcc scheme installed"));
		} catch (e) {
			console.error(e);
			toastThrownError(e)
		}
	}


	return (
		<main className="flex flex-col gap-2 flex-shrink overflow-y-auto flex-grow">
			<Card className={"flex-shrink-0 p-4"}>
				<h2 className={"pb-2"}>{tc("settings:unity hub path")}</h2>
				<div className={"flex gap-1"}>
					{
						settings.unity_hub
							? <Input className="flex-auto" value={settings.unity_hub} disabled/>
							: <Input value={"Unity Hub Not Found"} disabled className={"flex-auto text-destructive"}/>
					}
					<Button className={"flex-none px-4"} onClick={selectUnityHub}>{tc("general:button:select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<div className={"pb-2 flex align-middle"}>
					<div className={"flex-grow flex items-center"}>
						<h2>{tc("settings:unity installations")}</h2>
					</div>
					<Button onClick={addUnity} size={"sm"} className={"m-1"}>{tc("settings:button:add unity")}</Button>
				</div>
				<Card className="w-full overflow-x-auto overflow-y-scroll min-h-[20vh]">
					<CardHeader>
						<UnityTable unityPaths={settings.unity_paths}/>
					</CardHeader>
				</Card>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("settings:default project path")}</h2>
				<p className={"whitespace-normal"}>
					{tc("settings:default project path description")}
				</p>
				<div className={"flex gap-1"}>
					<Input className="flex-auto" value={settings.default_project_path} disabled/>
					<Button className={"flex-none px-4"}
									onClick={selectProjectDefaultFolder}>{tc("general:button:select")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("projects:backup")}</h2>
				<div className="mt-2">
					<h3>{tc("settings:backup:path")}</h3>
					<p className={"whitespace-normal"}>
						{tc("settings:backup:path description")}
					</p>
					<div className={"flex gap-1"}>
						<Input className="flex-auto" value={settings.project_backup_path} disabled/>
						<Button className={"flex-none px-4"}
										onClick={selectProjectBackupFolder}>{tc("general:button:select")}</Button>
					</div>
				</div>
				<div className="mt-2">
					<label className={"flex items-center"}>
						<h3>{tc("settings:backup:format")}</h3>
						<Select defaultValue={settings.backup_format} onValueChange={setBackupFormat}>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									<SelectItem value={"default"}>{tc("settings:backup:format:default")}</SelectItem>
									<SelectItem value={"zip-store"}>{tc("settings:backup:format:zip-store")}</SelectItem>
									<SelectItem value={"zip-fast"}>{tc("settings:backup:format:zip-fast")}</SelectItem>
									<SelectItem value={"zip-best"}>{tc("settings:backup:format:zip-best")}</SelectItem>
								</SelectGroup>
							</SelectContent>
						</Select>
					</label>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<p className={"whitespace-normal"}>
					{tc("settings:show prerelease description")}
				</p>
				<label className={"flex items-center"}>
					<div className={"p-3"}>
						<Checkbox checked={settings.show_prerelease_packages} onCheckedChange={(e) => toggleShowPrereleasePackages(e)}/>
					</div>
					{tc("settings:show prerelease")}
				</label>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<label className={"flex items-center"}>
					<h2>{tc("settings:language")}: </h2>
					{lang && (
						<Select defaultValue={lang} onValueChange={changeLanguage}>
							<SelectTrigger>
								<SelectValue/>
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									{
										languages.map((lang) => (
											<SelectItem key={lang} value={lang}>{tc("settings:langName", {lng: lang})}</SelectItem>
										))
									}
								</SelectGroup>
							</SelectContent>
						</Select>
					)}
				</label>
			</Card>
			{unityDialog}
			{unityHubDialog}
			{projectDefaultDialog}
			{projectBackupDialog}
			<Card className={"flex-shrink-0 p-4"}>
				<label className={"flex items-center"}>
					<h2>{tc("settings:theme")}: </h2>
					{theme && (
						<Select defaultValue={theme} onValueChange={changeTheme}>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									<SelectItem value={"system"}>{tc("settings:theme:system")}</SelectItem>
									<SelectItem value={"light"}>{tc("settings:theme:light")}</SelectItem>
									<SelectItem value={"dark"}>{tc("settings:theme:dark")}</SelectItem>
								</SelectGroup>
							</SelectContent>
						</Select>
					)}
				</label>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("settings:check update")}</h2>
				<div>
					<Button onClick={() => emit("tauri://update")}>{tc("settings:check update")}</Button>
				</div>
			</Card>
			{osType != "Darwin" && <Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("settings:vcc scheme")}</h2>
				<p className={"whitespace-normal"}>
					{tc("settings:vcc scheme description")}
				</p>
				<div>
					<Button onClick={installVccProtocol}>{tc("settings:register vcc scheme")}</Button>
				</div>
			</Card>}
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("settings:report issue")}</h2>
				<div>
					<Button onClick={reportIssue}>{tc("settings:button:open issue")}</Button>
				</div>
			</Card>
			<Card className={"flex-shrink-0 p-4"}>
				<h2>{tc("settings:licenses")}</h2>
				<p className={"whitespace-normal"}>
					{tc("settings:licenses description", {}, {
						components: {l: <Link href={"/settings/licenses"} className={"underline"}/>}
					})}
				</p>
			</Card>
		</main>
	)
}

function UnityTable(
	{
		unityPaths,
	}: {
		unityPaths: [path: string, version: string, fromHub: boolean][]
	}
) {
	const UNITY_TABLE_HEAD = ["settings:unity:version", "settings:unity:path", "general:source"];
	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				{UNITY_TABLE_HEAD.map((head, index) => (
					<th key={index}
							className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5`}>
						<small className="font-normal leading-none">{tc(head)}</small>
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
							{isFromHub ? tc("settings:unity:source:unity hub") : tc("settings:unity:source:manual")}
						</td>
					</tr>
				))
			}
			</tbody>
		</table>
	)
}
