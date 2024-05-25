import React, {Fragment, useState} from "react";
import {Button} from "@/components/ui/button";
import {Dialog, DialogContent, DialogDescription, DialogFooter, DialogTitle} from "@/components/ui/dialog";
import {tc, tt} from "@/lib/i18n";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {
	environmentCopyProjectForMigration,
	projectCallUnityForMigration,
	projectMigrateProjectTo2022, TauriUnityVersions
} from "@/lib/bindings";
import {callAsyncCommand} from "@/lib/call-async-command";
import {useRouter} from "next/navigation";
import {shellOpen} from "@/lib/shellOpen";
import {useUnitySelectorDialog} from "@/lib/use-unity-selector-dialog";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

function findRecommendedUnity(unityVersions?: TauriUnityVersions): UnityInstallation[] {
	if (unityVersions == null) return [];
	return unityVersions.unity_paths.filter(([_p, v, _]) => v == unityVersions.recommended_version);
}

export function useUnity2022Migration(
	{
		projectPath,
		unityVersions,
		refresh,
	}: {
		projectPath: string,
		unityVersions?: TauriUnityVersions,
		refresh?: () => void,
	}
): Result {
	return useMigrationInternal({
		projectPath,
		unityVersions,
		updateProjectPreUnityLaunch: async (project) => await projectMigrateProjectTo2022(project),
		refresh,
		ConfirmComponent: MigrationConfirmMigrationDialog,
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
		unityVersions,
		refresh,
	}: {
		projectPath: string,
		unityVersions?: TauriUnityVersions,
		refresh?: () => void,
	}
): Result {
	return useMigrationInternal({
		projectPath,
		unityVersions,
		updateProjectPreUnityLaunch: async () => {
		}, // nothing pre-launch
		refresh,

		ConfirmComponent: MigrationConfirmMigrationPatchDialog,
	});
}

function MigrationConfirmMigrationPatchDialog(
	{
		unity,
		cancel,
		doMigrate,
	}: {
		unity: string,
		cancel: () => void,
		doMigrate: (inPlace: boolean) => void,
	}) {
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

type StateInternal = {
	state: "normal";
} | {
	state: "confirm";
} | {
	state: "noExactUnity2022";
} | {
	state: "copyingProject";
} | {
	state: "updating";
} | {
	state: "finalizing";
	lines: [number, string][];
}

type Result = {
	dialog: React.ReactNode;
	request: () => void;
}

type ConfirmProps = {
	unity: string,
	cancel: () => void,
	doMigrate: (inPlace: boolean) => void,
}

function useMigrationInternal(
	{
		projectPath,
		unityVersions,
		updateProjectPreUnityLaunch,
		refresh,

		ConfirmComponent,
	}: {
		projectPath: string,
		unityVersions?: TauriUnityVersions,
		updateProjectPreUnityLaunch: (projectPath: string) => Promise<unknown>,
		refresh?: () => void,

		ConfirmComponent: React.ComponentType<ConfirmProps>,
	}
): Result {
	const router = useRouter();
	const unitySelector = useUnitySelectorDialog();

	const [installStatus, setInstallStatus] = React.useState<StateInternal>({state: "normal"});

	const request = async () => {
		const unityFound = findRecommendedUnity(unityVersions);
		if (unityFound.length == 0)
			setInstallStatus({state: "noExactUnity2022"});
		else
			setInstallStatus({state: "confirm"});
	}

	const startMigrateProjectTo2022 = async (inPlace: boolean) => {
		try {
			const unityFound = findRecommendedUnity(unityVersions);
			switch (unityFound.length) {
				case 0:
					setInstallStatus({state: "noExactUnity2022"});
					break;
				case 1:
					// noinspection ES6MissingAwait
					continueMigrateProjectTo2022(inPlace, unityFound[0][0]);
					break;
				default:
					const selected = await unitySelector.select(unityFound);
					if (selected == null)
						setInstallStatus({state: "normal"});
					else
						// noinspection ES6MissingAwait
						continueMigrateProjectTo2022(inPlace, selected);
					break;
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
			await updateProjectPreUnityLaunch(migrateProjectPath);
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

	const cancelMigrateProjectTo2022 = async () => {
		setInstallStatus({state: "normal"});
	}

	let dialogBodyForState: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			dialogBodyForState = null;
			break;
		case "confirm":
			dialogBodyForState = <ConfirmComponent
				unity={unityVersions!.recommended_version}
				cancel={cancelMigrateProjectTo2022}
				doMigrate={startMigrateProjectTo2022}
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
		dialog: <>
			{unitySelector.dialog}
			{dialogBodyForState == null ? null :
				<Dialog open>
					<DialogContent className={"whitespace-normal leading-relaxed"}>
						<DialogTitle>{tc("projects:manage:dialog:unity migrate header")}</DialogTitle>
						{dialogBodyForState}
					</DialogContent>
				</Dialog>}
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
		console.log("openUnityHub", installWithUnityHubLink)
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
