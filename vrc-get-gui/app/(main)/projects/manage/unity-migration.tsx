import React, {Fragment, useCallback} from "react";
import {Button} from "@/components/ui/button";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";
import {tc, tt} from "@/lib/i18n";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {
	environmentCopyProjectForMigration, environmentUnityVersions,
	projectCallUnityForMigration, projectIsUnityLaunching,
	projectMigrateProjectTo2022, TauriUnityVersions
} from "@/lib/bindings";
import {callAsyncCommand} from "@/lib/call-async-command";
import {useRouter} from "next/navigation";
import {shellOpen} from "@/lib/shellOpen";
import {useUnitySelectorDialog} from "@/lib/use-unity-selector-dialog";
import {assertNever} from "@/lib/assert-never";
import {compareUnityVersionString, parseUnityVersion} from "@/lib/version";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

function findRecommendedUnity(unityVersions: TauriUnityVersions): FindUnityResult {
	const versions = unityVersions.unity_paths.filter(([_p, v, _]) => v == unityVersions.recommended_version);

	if (versions.length == 0) {
		return {
			expectingVersion: unityVersions.recommended_version,
			installLink: unityVersions.install_recommended_version_link,
			found: false,
		};
	} else {
		return {
			expectingVersion: unityVersions.recommended_version,
			found: true,
			installations: versions,
		};
	}
}

export function useUnity2022Migration(
	{
		projectPath,
		refresh,
	}: {
		projectPath: string,
		refresh?: () => void,
	}
): Result<{}> {
	return useMigrationInternal({
		projectPath,
		updateProjectPreUnityLaunch: async (project) => await projectMigrateProjectTo2022(project),
		findUnity: findRecommendedUnity,
		refresh,
		ConfirmComponent: MigrationConfirmMigrationDialog,
		dialogHeader: tc("projects:manage:dialog:unity migrate header"),
	});
}

function MigrationConfirmMigrationDialog({cancel, doMigrate}: ConfirmProps) {
	return (
		<>
			<DialogDescription>
				<p className={"text-destructive"}>
					{tc("projects:dialog:vpm migrate description")}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate(false)} variant={"destructive"}
								className="mr-1">{tc("projects:button:migrate copy")}</Button>
				<Button onClick={() => doMigrate(true)} variant={"destructive"}>{tc("projects:button:migrate in-place")}</Button>
			</DialogFooter>
		</>
	);
}

export function useUnity2022PatchMigration(
	{
		projectPath,
		refresh,
	}: {
		projectPath: string,
		refresh?: () => void,
	}
): Result<{}> {
	return useMigrationInternal({
		projectPath,
		updateProjectPreUnityLaunch: async () => {
		}, // nothing pre-launch
		findUnity: findRecommendedUnity,
		refresh,

		ConfirmComponent: MigrationConfirmMigrationPatchDialog,
		dialogHeader: tc("projects:manage:dialog:unity migrate header"),
	});
}

function MigrationConfirmMigrationPatchDialog({result, cancel, doMigrate}: ConfirmProps) {
	const unity = result.expectingVersion;
	return (
		<>
			<DialogDescription>
				<p className={"text-destructive"}>
					{tc("projects:dialog:migrate unity2022 patch description", {unity})}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate(true)} variant={"destructive"}>{tc("projects:button:migrate in-place")}</Button>
			</DialogFooter>
		</>
	);
}

// endregion unity version change

export function useUnityVersionChange(
	{
		projectPath,
		refresh,
	}: {
		projectPath: string,
		refresh?: () => void,
	}
): Result<{ version: string, currentUnityVersion: string, isVRCProject: boolean }> {
	const use = useMigrationInternal({
		projectPath,
		updateProjectPreUnityLaunch: async (project, data) => {
			if (data.isVRC && data.kind == "upgradeMajor" && data.targetUnityVersion.startsWith("2022.")) {
				await projectMigrateProjectTo2022(project)
			}
		},
		findUnity: findUnityForUnityChange,
		refresh,
		ConfirmComponent: UnityVersionChange,
		dialogHeader: tc("projects:manage:dialog:unity change version header"),
	});

	const request = use.request;

	return {
		dialog: use.dialog,
		request: useCallback(({version, currentUnityVersion, isVRCProject}) => {
			if (currentUnityVersion == null) throw new Error("unexpected");
			const v = detectChangeUnityKind(currentUnityVersion, version, isVRCProject);
			request(v);
		}, [request]),
	};
}

function UnityVersionChange({cancel, doMigrate, data}: ConfirmProps<ChangeUnityData>) {
	// TODO: description

	let mainMessage: React.ReactNode;

	switch (data.kind) {
		case "downgradeMajor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc("projects:manage:dialog:downgrade major vrchat supported");
				} else {
					mainMessage = tc("projects:manage:dialog:downgrade major vrchat unsupported");
				}
			} else {
				mainMessage = tc("projects:manage:dialog:downgrade major generic");
			}
			break;
		case "downgradePatchOrMinor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc("projects:manage:dialog:downgrade minor vrchat supported");
				} else {
					mainMessage = tc("projects:manage:dialog:downgrade minor vrchat unsupported");
				}
			} else {
				mainMessage = tc("projects:manage:dialog:downgrade minor generic");
			}
			break;
		case "upgradePatchOrMinor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc("projects:manage:dialog:upgrade minor vrchat supported");
				} else {
					mainMessage = tc("projects:manage:dialog:upgrade minor vrchat unsupported");
				}
			} else {
				mainMessage = tc("projects:manage:dialog:upgrade minor generic");
			}
			break;
		case "upgradeMajor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc("projects:manage:dialog:upgrade major vrchat supported");
				} else {
					mainMessage = tc("projects:manage:dialog:upgrade major vrchat unsupported");
				}
			} else {
				mainMessage = tc("projects:manage:dialog:upgrade major generic");
			}
			break;
		default:
			assertNever(data.kind);
	}

	return (
		<>
			<DialogDescription>
				<p className={"text-destructive"}>
					{mainMessage}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate(true)} variant={"destructive"}>{tc("projects:button:change unity version")}</Button>
			</DialogFooter>
		</>
	);
}

type ChangeUnityKind = "downgradeMajor" | "downgradePatchOrMinor" | "upgradePatchOrMinor" | "upgradeMajor";

type ChangeUnityData = ({
	kind: ChangeUnityKind;
	isVRC: false;
} | {
	kind: ChangeUnityKind;
	isVRC: true;
	isTargetVersionSupportedByVRC: boolean;
}) & {
	targetUnityVersion: string,
}

function detectChangeUnityKind(currentVersion: string, targetUnityVersion: string, isVRCProject: boolean): ChangeUnityData {
	const parsedCurrent = parseUnityVersion(currentVersion)!;
	const parsedTarget = parseUnityVersion(targetUnityVersion)!;

	let kind: ChangeUnityData["kind"] =
		compareUnityVersionString(currentVersion, targetUnityVersion) >= 0 
			? (parsedCurrent.major == parsedTarget.major ? "downgradePatchOrMinor" : "downgradeMajor")
			: (parsedCurrent.major == parsedTarget.major ? "upgradePatchOrMinor" : "upgradeMajor");

	if (isVRCProject) {
		const supportedVersions = [
			"2019.4.31f1",
			"2022.3.6f1",
			"2022.3.22f1",
		]
		return {
			kind,
			isVRC: true,
			isTargetVersionSupportedByVRC: supportedVersions.includes(targetUnityVersion),
			targetUnityVersion,
		};
	} else {
		return {
			kind,
			isVRC: false,
			targetUnityVersion,
		};
	}
}

function findUnityForUnityChange(unityVersions: TauriUnityVersions, data: ChangeUnityData): FindUnityResult {
	let foundVersions = unityVersions.unity_paths.filter(([_p, v, _]) => v == data.targetUnityVersion);
	if (foundVersions.length == 0) throw new Error("unreachable");
	return {
		expectingVersion: data.targetUnityVersion,
		found: true,
		installations: foundVersions,
	};
}

// endregion

type StateInternal<Data> = {
	state: "normal";
} | {
	state: "confirm";
	data: Data;
	findResult: FindUnityResult & { found: true };
} | {
	state: "noExactUnity2022";
	data: Data;
	findResult: FindUnityResult & { found: false };
} | {
	state: "copyingProject";
} | {
	state: "updating";
} | {
	state: "finalizing";
	lines: [number, string][];
}

type Result<Data> = {
	dialog: React.ReactNode;
	request: (data: Data) => void;
}

type ConfirmProps<Data = {}> = {
	result: FindUnityResult,
	data: Data,
	cancel: () => void,
	doMigrate: (inPlace: boolean) => void,
}

type FindUnityResult = FindUnityFoundResult | FindUnityNotFoundResult

interface FindUnityFoundResult {
	expectingVersion: string;
	found: true,
	installations: UnityInstallation[];
}

interface FindUnityNotFoundResult {
	expectingVersion: string;
	installLink: string;
	found: false,
}

function useMigrationInternal<Data>(
	{
		projectPath,
		updateProjectPreUnityLaunch,
		findUnity,
		refresh,

		ConfirmComponent,
		dialogHeader,
	}: {
		projectPath: string,
		updateProjectPreUnityLaunch: (projectPath: string, data: Data) => Promise<unknown>,
		findUnity: (unityVersions: TauriUnityVersions, data: Data) => FindUnityResult,
		refresh?: () => void,

		ConfirmComponent: React.ComponentType<ConfirmProps<Data>>,
		dialogHeader: React.ReactNode,
	}
): Result<Data> {
	const router = useRouter();
	const unitySelector = useUnitySelectorDialog();

	const [installStatus, setInstallStatus] = React.useState<StateInternal<Data>>({state: "normal"});

	const request = async (data: Data) => {
		if (await projectIsUnityLaunching(projectPath)) {
			toastError(tt("projects:toast:close unity before migration"));
			return;
		}
		const unityVersions = await environmentUnityVersions();
		const findResult = findUnity(unityVersions, data);
		if (!findResult.found) {
			setInstallStatus({state: "noExactUnity2022", data, findResult});
		}
		else
			setInstallStatus({state: "confirm", data, findResult});
	}

	const startChangeUnityVersion = async (inPlace: boolean, unityFound: UnityInstallation[], data: Data) => {
		try {
			switch (unityFound.length) {
				case 0:
					throw new Error("unreachable");
				case 1:
					void continueChangeUnityVersion(inPlace, unityFound[0][0], data);
					break;
				default:
					const selected = await unitySelector.select(unityFound);
					if (selected == null)
						setInstallStatus({state: "normal"});
					else
						void continueChangeUnityVersion(inPlace, selected.unityPath, data);
					break;
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({state: "normal"});
		}
	}

	const continueChangeUnityVersion = async (inPlace: boolean, unityPath: string, data: Data) => {
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
			await updateProjectPreUnityLaunch(migrateProjectPath, data);
			setInstallStatus({state: "finalizing", lines: []});
			let lineNumber = 0;
			let [, promise] = callAsyncCommand(projectCallUnityForMigration, [migrateProjectPath, unityPath], lineString => {
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
					assertNever(finalizeResult);
			}
			if (inPlace) {
				setInstallStatus({state: "normal"});
				refresh?.();
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

	const cancelChangeUnityVersion = async () => {
		setInstallStatus({state: "normal"});
	}

	let dialogBodyForState: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			dialogBodyForState = null;
			break;
		case "confirm":
			dialogBodyForState = <ConfirmComponent
				result={installStatus.findResult}
				cancel={cancelChangeUnityVersion}
				data={installStatus.data}
				doMigrate={(inPlace) => startChangeUnityVersion(inPlace, installStatus.findResult.installations, installStatus.data)}
			/>;
			break;
		case "copyingProject":
			dialogBodyForState = <MigrationCopyingDialog/>;
			break
		case "updating":
			dialogBodyForState = <MigrationMigratingDialog/>;
			break;
		case "noExactUnity2022":
			dialogBodyForState = <NoExactUnity2022Dialog
				expectedVersion={installStatus.findResult.expectingVersion}
				installWithUnityHubLink={installStatus.findResult.installLink}
				close={cancelChangeUnityVersion}
			/>;
			break;
		case "finalizing":
			dialogBodyForState = <MigrationCallingUnityForMigrationDialog lines={installStatus.lines}/>;
			break;
		default:
			assertNever(installStatus);
	}

	return {
		dialog: <>
			{unitySelector.dialog}
			{dialogBodyForState == null ? null :
				<DialogOpen className={"whitespace-normal leading-relaxed"}>
					<DialogTitle>{dialogHeader}</DialogTitle>
					{dialogBodyForState}
				</DialogOpen>}
		</>,
		request,
	};
}

function MigrationCopyingDialog() {
	return <DialogDescription>
		<p>
			{tc("projects:pre-migrate copying...")}
		</p>
		<p>
			{tc("projects:manage:dialog:do not close")}
		</p>
	</DialogDescription>;
}

function MigrationMigratingDialog() {
	return <DialogDescription>
		<p>
			{tc("projects:migrating...")}
		</p>
		<p>
			{tc("projects:manage:dialog:do not close")}
		</p>
	</DialogDescription>;
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

	return <DialogDescription>
		<p>
			{tc("projects:manage:dialog:unity migrate finalizing...")}
		</p>
		<p>
			{tc("projects:manage:dialog:do not close")}
		</p>
		{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
		<pre className={"overflow-y-auto h-[50vh] bg-secondary text-secondary-foreground text-sm"}>
					{lines.map(([lineNumber, line]) => <Fragment key={lineNumber}>{line}{"\n"}</Fragment>)}
			<div ref={ref}/>
				</pre>
	</DialogDescription>;
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
		await shellOpen(installWithUnityHubLink);
	}

	return <>
		<DialogDescription>
			<p>
				{tc("projects:manage:dialog:exact version unity not found for patch migration description", {unity: expectedVersion})}
			</p>
		</DialogDescription>
		<DialogFooter className={"gap-2"}>
			<Button onClick={openUnityHub}>{tc("projects:manage:dialog:open unity hub")}</Button>
			<Button onClick={close} className="mr-1">{tc("general:button:close")}</Button>
		</DialogFooter>
	</>;
}
