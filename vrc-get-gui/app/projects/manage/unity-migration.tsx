import React, {Fragment, useState} from "react";
import {Button, Dialog, DialogBody, DialogFooter, DialogHeader, Radio, Typography} from "@material-tailwind/react";
import {nop} from "@/lib/nop";
import {tc, tt} from "@/lib/i18n";
import {parseUnityVersion} from "@/lib/version";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {
	environmentCopyProjectForMigration,
	projectCallUnityForMigration,
	projectMigrateProjectTo2022
} from "@/lib/bindings";
import {callAsyncCommand} from "@/lib/call-async-command";
import {useRouter} from "next/navigation";
import {shellOpen} from "@/lib/shellOpen";

type State2022 = {
	state: "normal";
} | {
	state: "confirm";
} | {
	state: "selectUnityVersion";
	versionMismatch: boolean;
	unityVersions: [path: string, version: string, fromHub: boolean][];
	inPlace: boolean;
} | {
	state: "copyingProject";
} | {
	state: "updating";
} | {
	state: "finalizing";
	lines: [number, string][];
}

type Result2022 = {
	dialog: React.ReactNode;
	requestMigrateProjectTo2022: () => void;
}

export function useUnity2022Migration(
	{
		projectPath,
		unityVersions,
	}: {
		projectPath: string,
		unityVersions?: {
			unity_paths: [string, string, boolean][],
			recommended_version: string,
		},
	}
): Result2022 {
	const router = useRouter();

	const [installStatus, setInstallStatus] = React.useState<State2022>({state: "normal"});

	type FindUnity2022Result = {
		type: "NoUnity2022";
	} | {
		type: "ExactMatches";
		paths: [path: string, version: string, fromHub: boolean][];
	} | {
		type: "NonExactMatches";
		paths: [path: string, version: string, fromHub: boolean][];
	}

	function findUnity2022ForMigration(): FindUnity2022Result | null {
		if (unityVersions == null) return null;
		const unity2022 = unityVersions.unity_paths.filter(([_p, v, _]) => parseUnityVersion(v)?.major == 2022);
		if (unity2022.length == 0) return {type: "NoUnity2022"};
		const exactMatches = unity2022.filter(([_p, v, _]) => v == unityVersions.recommended_version);
		if (exactMatches.length != 0) return {type: "ExactMatches", paths: exactMatches};
		return {type: "NonExactMatches", paths: unity2022};
	}

	const requestMigrateProjectTo2022 = async () => {
		setInstallStatus({state: "confirm"});
	}

	const startMigrateProjectTo2022 = async (inPlace: boolean) => {
		try {
			const findUnity2022Result = findUnity2022ForMigration();
			if (findUnity2022Result == null) throw new Error("unexpectedly null");
			switch (findUnity2022Result.type) {
				case "NoUnity2022":
					toastError(tt("projects:toast:unity migrate failed by unity not found"));
					setInstallStatus({state: "normal"});
					return;
				case "ExactMatches":
					if (findUnity2022Result.paths.length == 1) {
						// noinspection ES6MissingAwait
						continueMigrateProjectTo2022(inPlace, findUnity2022Result.paths[0][0]);
					} else {
						setInstallStatus({
							state: "selectUnityVersion",
							versionMismatch: false,
							unityVersions: findUnity2022Result.paths,
							inPlace,
						})
					}
					break;
				case "NonExactMatches":
					setInstallStatus({
						state: "selectUnityVersion",
						versionMismatch: true,
						unityVersions: findUnity2022Result.paths,
						inPlace,
					});
					break;
				default:
					const _: never = findUnity2022Result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({state: "normal"});
		}
	}

	const continueMigrateProjectTo2022 = async (inPlace: boolean, unityPath: string) => {
		try {
			let migrateProjectPath;
			if (inPlace) {
				migrateProjectPath = projectPath;
			} else {
				// copy
				setInstallStatus({state: "copyingProject"});
				migrateProjectPath = await environmentCopyProjectForMigration(projectPath);
			}
			setInstallStatus({state: "updating"});
			await projectMigrateProjectTo2022(migrateProjectPath);
			setInstallStatus({state: "finalizing", lines: []});
			let lineNumber = 0;
			let [__, promise] = callAsyncCommand(projectCallUnityForMigration, [migrateProjectPath, unityPath], lineString => {
				setInstallStatus(prev => {
					if (prev.state != "finalizing") return prev;
					lineNumber++;
					let line: [number, string] = [lineNumber, lineString];
					if (prev.lines.length > 200) {
						return {...prev, lines: [...prev.lines.slice(1), line]};
					} else {
						return {...prev, lines: [...prev.lines, line]};
					}
				})
			});
			const finalizeResult = await promise;
			if (finalizeResult == 'cancelled') {
				throw new Error("unexpectedly cancelled");
			}
			switch (finalizeResult.type) {
				case "ExistsWithNonZero":
					toastError(tt("projects:toast:unity exits with non-zero"));
					break;
				case "FinishedSuccessfully":
					toastSuccess(tt("projects:toast:unity migrated"));
					break;
				default:
					const _: never = finalizeResult;
			}
			if (inPlace) {
				setInstallStatus({state: "normal"});
				router.refresh();
			} else {
				setInstallStatus({state: "normal"});
				router.replace(`/projects/manage?${new URLSearchParams({projectPath: migrateProjectPath})}`);
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({state: "normal"});
		}
	};

	const cancelMigrateProjectTo2022 = async () => {
		setInstallStatus({state: "normal"});
	}

	let dialogBodyForState: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			dialogBodyForState = null;
			break;
		case "confirm":
			dialogBodyForState = <MigrationConfirmMigrationDialog
				cancel={cancelMigrateProjectTo2022}
				doMigrate={(inPlace) => startMigrateProjectTo2022(inPlace)}
			/>;
			break;
		case "selectUnityVersion":
			dialogBodyForState = <MigrationSelectUnityVersionDialog
				dueToMismatch={installStatus.versionMismatch}
				unityVersions={installStatus.unityVersions}
				cancel={cancelMigrateProjectTo2022}
				doMigrate={(unityPath) => continueMigrateProjectTo2022(installStatus.inPlace, unityPath)}
			/>;
			break;
		case "copyingProject":
			dialogBodyForState = <MigrationCopyingDialog/>;
			break
		case "updating":
			dialogBodyForState = <MigrationMigratingDialog/>;
			break;
		case "finalizing":
			dialogBodyForState = <MigrationCallingUnityForMigrationDialog lines={installStatus.lines}/>;
			break;
		default:
			const _: never = installStatus;
	}

	return {
		dialog: dialogBodyForState == null ? null : <Dialog open handler={nop} className={"whitespace-normal"}>
			<DialogHeader>{tc("projects:manage:dialog:unity migrate header")}</DialogHeader>
			{dialogBodyForState}
		</Dialog>,
		requestMigrateProjectTo2022,
	};
}


function MigrationConfirmMigrationDialog(
	{
		cancel,
		doMigrate,
	}: {
		cancel: () => void,
		doMigrate: (inPlace: boolean) => void,
	}) {
	return (
		<>
			<DialogBody>
				<Typography className={"text-red-700"}>
					{tc("projects:dialog:vpm migrate description")}
				</Typography>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate(false)} color={"red"}
								className="mr-1">{tc("projects:button:migrate copy")}</Button>
				<Button onClick={() => doMigrate(true)} color={"red"}>{tc("projects:button:migrate in-place")}</Button>
			</DialogFooter>
		</>
	);
}

function MigrationSelectUnityVersionDialog(
	{
		dueToMismatch,
		unityVersions,
		cancel,
		doMigrate,
	}: {
		dueToMismatch: boolean,
		unityVersions: [path: string, version: string, boolean][],
		cancel: () => void,
		doMigrate: (unityPath: string) => void,
	}) {
	const name = useState(() => `unity2022migration-select-unity-version-${Math.random().toString(36).slice(2)}-radio`)[0];

	const [selectedUnityPath, setSelectedUnityPath] = useState<string | null>(null);

	return (
		<>
			<DialogBody>
				<Typography>
					{dueToMismatch
						? tc("projects:manage:dialog:exact version unity not found")
						: tc("projects:manage:dialog:multiple unity found")}
				</Typography>
				<Typography>
					{dueToMismatch && tc("projects:manage:dialog:exact version unity not found description")}
				</Typography>
				{unityVersions.map(([path, version, _]) =>
					<Radio
						key={path} name={name} label={`${version} (${path})`}
						checked={selectedUnityPath == path}
						onChange={() => setSelectedUnityPath(path)}
					/>)}
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button
					onClick={() => doMigrate(selectedUnityPath!)} color={"red"}
					disabled={selectedUnityPath == null}
				>{tc("projects:manage:button:continue")}</Button>
			</DialogFooter>
		</>
	);
}

function MigrationCopyingDialog() {
	return <DialogBody>
		<Typography>
			{tc("projects:pre-migrate copying...")}
		</Typography>
		<Typography>
			{tc("projects:manage:dialog:do not close")}
		</Typography>
	</DialogBody>;
}

function MigrationMigratingDialog() {
	return <DialogBody>
		<Typography>
			{tc("projects:migrating...")}
		</Typography>
		<Typography>
			{tc("projects:manage:dialog:do not close")}
		</Typography>
	</DialogBody>;
}

function MigrationCallingUnityForMigrationDialog(
	{
		lines
	}: {
		lines: [number, string][]
	}
) {
	const ref = React.useRef<HTMLDivElement>(null);

	React.useEffect(() => {
		ref.current?.scrollIntoView({behavior: "auto"});
	}, [lines]);

	return <DialogBody>
		<Typography>
			{tc("projects:manage:dialog:unity migrate finalizing...")}
		</Typography>
		<Typography>
			{tc("projects:manage:dialog:do not close")}
		</Typography>
		<pre className={"overflow-y-auto h-[50vh] bg-gray-900 text-white text-sm"}>
					{lines.map(([lineNumber, line]) => <Fragment key={lineNumber}>{line}{"\n"}</Fragment>)}
			<div ref={ref}/>
				</pre>
	</DialogBody>;
}


type State2022Patch = {
	state: "normal";
} | {
	state: "confirm";
} | {
	state: "selectUnityVersion";
	unityVersions: [path: string, version: string, fromHub: boolean][];
} | {
	state: "noExactUnity2022";
} | {
	state: "finalizing";
	lines: [number, string][];
}

type Result2022Patch = {
	dialog: React.ReactNode;
	requestMigrate: () => void;
}

export function useUnity2022PatchMigration(
	{
		projectPath,
		unityVersions,
	}: {
		projectPath: string,
		unityVersions?: {
			unity_paths: [string, string, boolean][],
			recommended_version: string,
			install_recommended_version_link: string,
		},
	}
): Result2022Patch {
	const router = useRouter();

	const [installStatus, setInstallStatus] = React.useState<State2022Patch>({state: "normal"});

	type FindUnity2022Result = {
		type: "NoUnity2022";
	} | {
		type: "ExactMatches";
		paths: [path: string, version: string, fromHub: boolean][];
	} | {
		type: "NotFound";
	}

	function findUnity2022ForPatchMigration(): FindUnity2022Result | null {
		if (unityVersions == null) return null;
		const unity2022 = unityVersions.unity_paths.filter(([_p, v, _]) => parseUnityVersion(v)?.major == 2022);
		if (unity2022.length == 0) return {type: "NoUnity2022"};
		const exactMatches = unity2022.filter(([_p, v, _]) => v == unityVersions.recommended_version);
		if (exactMatches.length != 0) return {type: "ExactMatches", paths: exactMatches};
		return {type: "NotFound"};
	}

	const requestMigrate = async () => {
		setInstallStatus({state: "confirm"});
	}

	const startMigrateProjectTo2022 = async () => {
		try {
			const findUnity2022Result = findUnity2022ForPatchMigration();
			if (findUnity2022Result == null) throw new Error("unexpectedly null");
			switch (findUnity2022Result.type) {
				case "NoUnity2022":
					toastError(tt("projects:toast:unity migrate failed by unity not found"));
					setInstallStatus({state: "normal"});
					return;
				case "ExactMatches":
					if (findUnity2022Result.paths.length == 1) {
						// noinspection ES6MissingAwait
						continueMigrateProjectTo2022(findUnity2022Result.paths[0][0]);
					} else {
						setInstallStatus({
							state: "selectUnityVersion",
							unityVersions: findUnity2022Result.paths,
						})
					}
					break;
				case "NotFound":
					setInstallStatus({state: "noExactUnity2022"});
					break;
				default:
					const _: never = findUnity2022Result;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({state: "normal"});
		}
	}

	const continueMigrateProjectTo2022 = async (unityPath: string) => {
		try {
			let migrateProjectPath = projectPath;
			setInstallStatus({state: "finalizing", lines: []});
			let lineNumber = 0;
			let [__, promise] = callAsyncCommand(projectCallUnityForMigration, [migrateProjectPath, unityPath], lineString => {
				setInstallStatus(prev => {
					if (prev.state != "finalizing") return prev;
					lineNumber++;
					let line: [number, string] = [lineNumber, lineString];
					if (prev.lines.length > 200) {
						return {...prev, lines: [...prev.lines.slice(1), line]};
					} else {
						return {...prev, lines: [...prev.lines, line]};
					}
				})
			});
			const finalizeResult = await promise;
			if (finalizeResult == 'cancelled') {
				throw new Error("unexpectedly cancelled");
			}
			switch (finalizeResult.type) {
				case "ExistsWithNonZero":
					toastError(tt("projects:toast:unity exits with non-zero"));
					break;
				case "FinishedSuccessfully":
					toastSuccess(tt("projects:toast:unity migrated"));
					break;
				default:
					const _: never = finalizeResult;
			}
			setInstallStatus({state: "normal"});
			router.refresh();
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({state: "normal"});
		}
	};

	const cancelMigrateProjectTo2022 = async () => {
		setInstallStatus({state: "normal"});
	}

	let dialogBodyForState: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			dialogBodyForState = null;
			break;
		case "confirm":
			dialogBodyForState = <MigrationConfirmMigrationPatchDialog
				unity={unityVersions!.recommended_version}
				cancel={cancelMigrateProjectTo2022}
				doMigrate={startMigrateProjectTo2022}
			/>;
			break;
		case "selectUnityVersion":
			dialogBodyForState = <MigrationSelectUnityVersionDialog
				dueToMismatch={false}
				unityVersions={installStatus.unityVersions}
				cancel={cancelMigrateProjectTo2022}
				doMigrate={(unityPath) => continueMigrateProjectTo2022(unityPath)}
			/>;
			break;
		case "noExactUnity2022":
			dialogBodyForState = <NoExactUnity2022Dialog
				expectedVersion={unityVersions!.recommended_version}
				installWithUnityHubLink={unityVersions!.install_recommended_version_link}
				close={cancelMigrateProjectTo2022}
			/>;
			break;
		case "finalizing":
			dialogBodyForState = <MigrationCallingUnityForMigrationDialog lines={installStatus.lines}/>;
			break;
		default:
			const _: never = installStatus;
	}

	return {
		dialog: dialogBodyForState == null ? null : <Dialog open handler={nop} className={"whitespace-normal"}>
			<DialogHeader>{tc("projects:manage:dialog:unity migrate header")}</DialogHeader>
			{dialogBodyForState}
		</Dialog>,
		requestMigrate,
	};
}

function MigrationConfirmMigrationPatchDialog(
	{
		unity,
		cancel,
		doMigrate,
	}: {
		unity: string,
		cancel: () => void,
		doMigrate: () => void,
	}) {
	return (
		<>
			<DialogBody>
				<Typography className={"text-red-700"}>
					{tc("projects:dialog:migrate unity2022 patch description", {unity})}
				</Typography>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate()} color={"red"}>{tc("projects:button:migrate in-place")}</Button>
			</DialogFooter>
		</>
	);
}

function NoExactUnity2022Dialog(
	{
		expectedVersion,
		installWithUnityHubLink,
		close,
	}: {
		expectedVersion: string,
		installWithUnityHubLink: string,
		close: () => void
	}) {
	const openUnityHub = async () => {
		console.log("openUnityHub", installWithUnityHubLink)
		await shellOpen(installWithUnityHubLink);
	}

	return <>
		<DialogBody>
			<Typography>
				{tc("projects:manage:dialog:exact version unity not found for patch migration description", {unity: expectedVersion})}
			</Typography>
		</DialogBody>
		<DialogFooter className={"gap-2"}>
			<Button onClick={openUnityHub}>{tc("projects:manage:dialog:open unity hub")}</Button>
			<Button onClick={close} className="mr-1">{tc("general:button:close")}</Button>
		</DialogFooter>
	</>;
}
