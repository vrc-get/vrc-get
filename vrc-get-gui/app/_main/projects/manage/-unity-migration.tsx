import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriCopyProjectForMigrationProgress,
	TauriCreateBackupProgress,
	TauriUnityVersions,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { tc, tt } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useUnitySelectorDialog } from "@/lib/use-unity-selector-dialog";
import { compareUnityVersionString, parseUnityVersion } from "@/lib/version";
import { useNavigate } from "@tanstack/react-router";
import React, { Fragment, useCallback } from "react";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

function MigrationConfirmMigrationDialog({ cancel, doMigrate }: ConfirmProps) {
	return (
		<>
			<DialogDescription>
				<p>{tc("projects:dialog:vpm migrate description")}</p>
			</DialogDescription>
			<DialogFooter className={"gap-1"}>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={() => doMigrate("backupArchive")}>
					{tc("projects:button:backup and migrate")}
				</Button>
				<Button onClick={() => doMigrate("copy")}>
					{tc("projects:button:migrate copy")}
				</Button>
				<Button onClick={() => doMigrate("none")} variant={"destructive"}>
					{tc("projects:button:migrate in-place")}
				</Button>
			</DialogFooter>
		</>
	);
}

function MigrationConfirmMigrationPatchDialog({
	result,
	cancel,
	doMigrate,
}: ConfirmProps) {
	const unity = result.expectingVersion;
	return (
		<>
			<DialogDescription>
				<p className={"text-destructive"}>
					{tc("projects:dialog:migrate unity2022 patch description", { unity })}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => doMigrate("none")} variant={"destructive"}>
					{tc("projects:button:migrate in-place")}
				</Button>
			</DialogFooter>
		</>
	);
}

// endregion unity version change

export function useUnityVersionChange({
	projectPath,
	refresh,
}: {
	projectPath: string;
	refresh?: () => void;
}): Result<{
	version: string;
	currentUnityVersion: string;
	isVRCProject: boolean;
	mayUseChinaVariant?: boolean;
}> {
	const use = useMigrationInternal({
		projectPath,
		updateProjectPreUnityLaunch: async (project, data) => {
			if (
				data.isVRC &&
				data.kind === "upgradeMajor" &&
				data.targetUnityVersion.startsWith("2022.")
			) {
				await commands.projectMigrateProjectTo2022(project);
			}
		},
		findUnity: findUnityForUnityChange,
		refresh,
		ConfirmComponent: UnityVersionChange,
		dialogHeader: (data) => {
			if (data.isVRC && data.isTargetVersionSupportedByVRC) {
				switch (data.kind) {
					case "upgradePatchOrMinor":
					case "upgradeMajor":
						return tc("projects:manage:dialog:unity migrate header");
				}
			}

			return tc("projects:manage:dialog:unity change version header");
		},
	});

	const request = use.request;

	return {
		dialog: use.dialog,
		request: useCallback(
			({ version, currentUnityVersion, isVRCProject, mayUseChinaVariant }) => {
				if (currentUnityVersion == null) throw new Error("unexpected");
				const v = detectChangeUnityKind(
					currentUnityVersion,
					version,
					isVRCProject,
					mayUseChinaVariant ?? false,
				);
				request(v);
			},
			[request],
		),
	};
}

function UnityVersionChange({
	cancel,
	doMigrate,
	data,
	result,
}: ConfirmProps<ChangeUnityData>) {
	// TODO: description

	if (data.isVRC && data.isTargetVersionSupportedByVRC) {
		// for supported migrations, show dialog same as migration
		switch (data.kind) {
			case "upgradePatchOrMinor":
				return (
					<MigrationConfirmMigrationPatchDialog
						cancel={cancel}
						doMigrate={doMigrate}
						result={result}
						data={{}}
					/>
				);
			case "upgradeMajor":
				return (
					<MigrationConfirmMigrationDialog
						cancel={cancel}
						doMigrate={doMigrate}
						result={result}
						data={{}}
					/>
				);
		}
	}

	let mainMessage: React.ReactNode;

	switch (data.kind) {
		case "downgradeMajor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc([
						"projects:manage:dialog:downgrade major vrchat supported",
						"projects:manage:dialog:downgrade major",
					]);
				} else {
					mainMessage = tc([
						"projects:manage:dialog:downgrade major vrchat unsupported",
						"projects:manage:dialog:downgrade major",
					]);
				}
			} else {
				mainMessage = tc("projects:manage:dialog:downgrade major");
			}
			break;
		case "downgradePatchOrMinor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc([
						"projects:manage:dialog:downgrade minor vrchat supported",
						"projects:manage:dialog:downgrade minor",
					]);
				} else {
					mainMessage = tc([
						"projects:manage:dialog:downgrade minor vrchat unsupported",
						"projects:manage:dialog:downgrade minor",
					]);
				}
			} else {
				mainMessage = tc("projects:manage:dialog:downgrade minor");
			}
			break;
		case "upgradePatchOrMinor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc([
						"projects:manage:dialog:upgrade minor vrchat supported",
						"projects:manage:dialog:upgrade minor",
					]);
				} else {
					mainMessage = tc([
						"projects:manage:dialog:upgrade minor vrchat unsupported",
						"projects:manage:dialog:upgrade minor",
					]);
				}
			} else {
				mainMessage = tc("projects:manage:dialog:upgrade minor");
			}
			break;
		case "upgradeMajor":
			if (data.isVRC) {
				if (data.isTargetVersionSupportedByVRC) {
					mainMessage = tc([
						"projects:manage:dialog:upgrade major vrchat supported",
						"projects:manage:dialog:upgrade major",
					]);
				} else {
					mainMessage = tc([
						"projects:manage:dialog:upgrade major vrchat unsupported",
						"projects:manage:dialog:upgrade major",
					]);
				}
			} else {
				mainMessage = tc("projects:manage:dialog:upgrade major");
			}
			break;
		case "changeChina":
			mainMessage = tc("projects:manage:dialog:changing china releases");
			break;
		default:
			assertNever(data.kind);
	}

	return (
		<>
			<DialogDescription>
				<p className={"text-destructive"}>{mainMessage}</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => doMigrate("none")} variant={"destructive"}>
					{tc("projects:button:change unity version")}
				</Button>
			</DialogFooter>
		</>
	);
}

type ChangeUnityKind =
	| "changeChina" // Changing between 'c' releases and non 'c' releases
	| "downgradeMajor"
	| "downgradePatchOrMinor"
	| "upgradePatchOrMinor"
	| "upgradeMajor";

type ChangeUnityData = (
	| {
			kind: ChangeUnityKind;
			isVRC: false;
	  }
	| {
			kind: ChangeUnityKind;
			isVRC: true;
			isTargetVersionSupportedByVRC: boolean;
	  }
) & {
	targetUnityVersion: string;
	mayUseChinaVariant: boolean;
};

function detectChangeUnityKind(
	currentVersion: string,
	targetUnityVersion: string,
	isVRCProject: boolean,
	mayUseChinaVariant: boolean,
): ChangeUnityData {
	// biome-ignore lint/style/noNonNullAssertion: the version is known to be valid
	const parsedCurrent = parseUnityVersion(currentVersion)!;
	// biome-ignore lint/style/noNonNullAssertion: the version is known to be valid
	const parsedTarget = parseUnityVersion(targetUnityVersion)!;

	const cmp = compareUnityVersionString(currentVersion, targetUnityVersion);
	const majorOrMinor =
		parsedCurrent.major === parsedTarget.major ? "PatchOrMinor" : "Major";

	const kind: ChangeUnityData["kind"] =
		cmp === 0
			? "changeChina"
			: cmp > 0
				? `downgrade${majorOrMinor}`
				: `upgrade${majorOrMinor}`;

	if (isVRCProject) {
		return {
			kind,
			isVRC: true,
			isTargetVersionSupportedByVRC:
				VRCSDK_UNITY_VERSIONS.includes(targetUnityVersion),
			targetUnityVersion,
			mayUseChinaVariant,
		};
	} else {
		return {
			kind,
			isVRC: false,
			targetUnityVersion,
			mayUseChinaVariant,
		};
	}
}

function findUnityForUnityChange(
	unityVersions: TauriUnityVersions,
	data: ChangeUnityData,
): FindUnityResult {
	let foundVersions = unityVersions.unity_paths.filter(
		([_p, v, _]) => v === data.targetUnityVersion,
	);
	// if international version not found, try to find china version
	if (
		foundVersions.length === 0 &&
		data.mayUseChinaVariant &&
		parseUnityVersion(data.targetUnityVersion)?.chinaIncrement == null
	) {
		const chinaVersion = `${data.targetUnityVersion}c1`;
		foundVersions = unityVersions.unity_paths.filter(
			([_p, v, _]) => v === chinaVersion,
		);
	}
	if (foundVersions.length === 0) {
		if (
			compareUnityVersionString(
				data.targetUnityVersion,
				unityVersions.recommended_version,
			) === 0
		) {
			return {
				expectingVersion: data.targetUnityVersion,
				// This is using link to international version but china version of hub will handle international to china conversion
				installLink: unityVersions.install_recommended_version_link,
				found: false,
			};
		} else {
			return {
				expectingVersion: data.targetUnityVersion,
				found: false,
			};
		}
	}
	return {
		expectingVersion: data.targetUnityVersion,
		found: true,
		installations: foundVersions,
	};
}

// endregion

type StateInternal<Data> =
	| {
			state: "normal";
	  }
	| {
			state: "confirm";
			data: Data;
			findResult: FindUnityResult & { found: true };
	  }
	| {
			state: "noExactUnity2022";
			data: Data;
			findResult: FindUnityResult & { found: false };
	  }
	| {
			state: "copyingProject";
			data: Data;
			progress: TauriCopyProjectForMigrationProgress;
	  }
	| {
			state: "backingUpProject";
			data: Data;
			progress: TauriCreateBackupProgress;
	  }
	| {
			state: "updating";
			data: Data;
	  }
	| {
			state: "finalizing";
			data: Data;
			lines: [number, string][];
	  };

type Result<Data> = {
	dialog: React.ReactNode;
	request: (data: Data) => void;
};

type ConfirmProps<Data = Record<string, never>> = {
	result: FindUnityResult;
	data: Data;
	cancel: () => void;
	doMigrate: (backupType: ProjectBackupType) => void;
};

type FindUnityResult = FindUnityFoundResult | FindUnityNotFoundResult;

type ProjectBackupType = "none" | "copy" | "backupArchive";

interface FindUnityFoundResult {
	expectingVersion: string;
	found: true;
	installations: UnityInstallation[];
}

interface FindUnityNotFoundResult {
	expectingVersion: string;
	installLink?: string;
	found: false;
}

function useMigrationInternal<Data>({
	projectPath,
	updateProjectPreUnityLaunch,
	findUnity,
	refresh,

	ConfirmComponent,
	dialogHeader,
}: {
	projectPath: string;
	updateProjectPreUnityLaunch: (
		projectPath: string,
		data: Data,
	) => Promise<unknown>;
	findUnity: (unityVersions: TauriUnityVersions, data: Data) => FindUnityResult;
	refresh?: () => void;

	ConfirmComponent: React.ComponentType<ConfirmProps<Data>>;
	dialogHeader: (data: Data) => React.ReactNode;
}): Result<Data> {
	const navigate = useNavigate();
	const unitySelector = useUnitySelectorDialog();

	const [installStatus, setInstallStatus] = React.useState<StateInternal<Data>>(
		{ state: "normal" },
	);

	const request = async (data: Data) => {
		if (await commands.projectIsUnityLaunching(projectPath)) {
			toastError(tt("projects:toast:close unity before migration"));
			return;
		}
		const unityVersions = await commands.environmentUnityVersions();
		const findResult = findUnity(unityVersions, data);
		if (!findResult.found) {
			setInstallStatus({ state: "noExactUnity2022", data, findResult });
		} else setInstallStatus({ state: "confirm", data, findResult });
	};

	const startChangeUnityVersion = async (
		backupType: ProjectBackupType,
		unityFound: UnityInstallation[],
		data: Data,
	) => {
		try {
			switch (unityFound.length) {
				case 0:
					throw new Error("unreachable");
				case 1:
					void continueChangeUnityVersion(backupType, unityFound[0][0], data);
					break;
				default: {
					const selected = await unitySelector.select(unityFound);
					if (selected == null) setInstallStatus({ state: "normal" });
					else
						void continueChangeUnityVersion(
							backupType,
							selected.unityPath,
							data,
						);
					break;
				}
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({ state: "normal" });
		}
	};

	const continueChangeUnityVersion = async (
		backupType: ProjectBackupType,
		unityPath: string,
		data: Data,
	) => {
		try {
			let migrateProjectPath: string;
			switch (backupType) {
				case "none":
					migrateProjectPath = projectPath;
					break;
				case "copy": {
					setInstallStatus({
						state: "copyingProject",
						data,
						progress: {
							proceed: 0,
							total: 1,
							last_proceed: "Collecting files...",
						},
					});
					const [, promise] = callAsyncCommand(
						commands.environmentCopyProjectForMigration,
						[projectPath],
						(progress) => {
							setInstallStatus((prev) => {
								if (prev.state !== "copyingProject") return prev;
								if (prev.progress.proceed > progress.proceed) return prev;
								return { ...prev, progress };
							});
						},
					);
					migrateProjectPath = await promise;
					break;
				}
				case "backupArchive": {
					setInstallStatus({
						state: "backingUpProject",
						data,
						progress: {
							proceed: 0,
							total: 1,
							last_proceed: "Collecting files...",
						},
					});
					const [, promise] = callAsyncCommand(
						commands.projectCreateBackup,
						[projectPath],
						(progress) => {
							setInstallStatus((prev) => {
								if (prev.state !== "backingUpProject") return prev;
								if (prev.progress.proceed > progress.proceed) return prev;
								return { ...prev, progress };
							});
						},
					);
					await promise;
					migrateProjectPath = projectPath;
					break;
				}
				default:
					assertNever(backupType);
			}
			setInstallStatus({ state: "updating", data });
			await updateProjectPreUnityLaunch(migrateProjectPath, data);
			setInstallStatus({ state: "finalizing", lines: [], data });
			let lineNumber = 0;
			const [, promise] = callAsyncCommand(
				commands.projectCallUnityForMigration,
				[migrateProjectPath, unityPath],
				(lineString) => {
					setInstallStatus((prev) => {
						if (prev.state !== "finalizing") return prev;
						lineNumber++;
						const line: [number, string] = [lineNumber, lineString];
						if (prev.lines.length > 200) {
							return { ...prev, lines: [...prev.lines.slice(1), line] };
						} else {
							return { ...prev, lines: [...prev.lines, line] };
						}
					});
				},
			);
			const finalizeResult = await promise;
			if (finalizeResult === "cancelled") {
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
			setInstallStatus({ state: "normal" });
			if (migrateProjectPath === projectPath) {
				refresh?.();
			} else {
				navigate({
					replace: true,
					to: "/projects/manage",
					search: { projectPath: migrateProjectPath },
				});
			}
		} catch (e) {
			console.error(e);
			toastThrownError(e);
			setInstallStatus({ state: "normal" });
		}
	};

	const cancelChangeUnityVersion = async () => {
		setInstallStatus({ state: "normal" });
	};

	let dialogHeaderForState: React.ReactNode = null;
	let dialogBodyForState: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			dialogBodyForState = null;
			break;
		case "confirm":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = (
				<ConfirmComponent
					result={installStatus.findResult}
					cancel={cancelChangeUnityVersion}
					data={installStatus.data}
					doMigrate={(backupType) =>
						startChangeUnityVersion(
							backupType,
							installStatus.findResult.installations,
							installStatus.data,
						)
					}
				/>
			);
			break;
		case "copyingProject":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = (
				<MigrationCopyingDialog progress={installStatus.progress} />
			);
			break;
		case "backingUpProject":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = (
				<MigrationBackingUpDialog progress={installStatus.progress} />
			);
			break;
		case "updating":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = <MigrationMigratingDialog />;
			break;
		case "noExactUnity2022":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = (
				<NoExactUnity2022Dialog
					expectedVersion={installStatus.findResult.expectingVersion}
					installWithUnityHubLink={installStatus.findResult.installLink}
					close={cancelChangeUnityVersion}
				/>
			);
			break;
		case "finalizing":
			dialogHeaderForState = dialogHeader(installStatus.data);
			dialogBodyForState = (
				<MigrationCallingUnityForMigrationDialog lines={installStatus.lines} />
			);
			break;
		default:
			assertNever(installStatus);
	}

	return {
		dialog: (
			<>
				{unitySelector.dialog}
				{dialogBodyForState == null ? null : (
					<DialogOpen className={"whitespace-normal leading-relaxed"}>
						<DialogTitle>{dialogHeaderForState}</DialogTitle>
						{dialogBodyForState}
					</DialogOpen>
				)}
			</>
		),
		request,
	};
}

function MigrationCopyingDialog({
	progress,
}: {
	progress: TauriCopyProjectForMigrationProgress;
}) {
	return (
		<DialogDescription>
			<p>{tc("projects:pre-migrate copying...")}</p>
			<p>
				{tc("projects:dialog:proceed k/n", {
					count: progress.proceed,
					total: progress.total,
				})}
			</p>
			<Progress value={progress.proceed} max={progress.total} />
			<p>{tc("projects:do not close")}</p>
		</DialogDescription>
	);
}

function MigrationBackingUpDialog({
	progress,
}: {
	progress: TauriCreateBackupProgress;
}) {
	return (
		<DialogDescription>
			<p>{tc("projects:dialog:creating backup...")}</p>
			<p>
				{tc("projects:dialog:proceed k/n", {
					count: progress.proceed,
					total: progress.total,
				})}
			</p>
			<Progress value={progress.proceed} max={progress.total} />
			<p>{tc("projects:do not close")}</p>
		</DialogDescription>
	);
}

function MigrationMigratingDialog() {
	return (
		<DialogDescription>
			<p>{tc("projects:migrating...")}</p>
			<p>{tc("projects:do not close")}</p>
		</DialogDescription>
	);
}

function MigrationCallingUnityForMigrationDialog({
	lines,
}: {
	lines: [number, string][];
}) {
	const ref = React.useRef<HTMLDivElement>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to scroll to bottom on lines changed
	React.useEffect(() => {
		ref.current?.scrollIntoView({ behavior: "auto" });
	}, [lines]);

	return (
		<DialogDescription>
			<p>{tc("projects:manage:dialog:unity migrate finalizing...")}</p>
			<p>{tc("projects:do not close")}</p>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<pre
				className={
					"overflow-y-auto h-[50vh] bg-secondary text-secondary-foreground text-sm"
				}
			>
				{lines.map(([lineNumber, line]) => (
					<Fragment key={lineNumber}>
						{line}
						{"\n"}
					</Fragment>
				))}
				<div ref={ref} />
			</pre>
		</DialogDescription>
	);
}

function NoExactUnity2022Dialog({
	expectedVersion,
	installWithUnityHubLink,
	close,
}: {
	expectedVersion: string;
	installWithUnityHubLink?: string;
	close: () => void;
}) {
	const openUnityHub = async () => {
		if (installWithUnityHubLink != null)
			await commands.utilOpenUrl(installWithUnityHubLink);
	};

	return (
		<>
			<DialogDescription>
				<p>
					{tc(
						"projects:manage:dialog:exact version unity not found for patch migration description",
						{ unity: expectedVersion },
					)}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				{installWithUnityHubLink && (
					<Button onClick={openUnityHub}>
						{tc("projects:dialog:open unity hub")}
					</Button>
				)}
				<Button onClick={close} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</>
	);
}
