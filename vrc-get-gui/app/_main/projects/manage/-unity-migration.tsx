import { BackupProjectDialog } from "@/components/BackupProjectDialog";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { UnitySelectorDialog } from "@/components/unity-selector-dialog";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriCallUnityForMigrationResult,
	TauriCopyProjectProgress,
	TauriUnityVersions,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { VRCSDK_UNITY_VERSIONS } from "@/lib/constants";
import { type DialogContext, openSingleDialog, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { compareUnityVersionString, parseUnityVersion } from "@/lib/version";
import type { NavigateFn } from "@tanstack/react-router";
import React, { Fragment, useEffect, useState } from "react";

export async function unityVersionChange({
	version: targetUnityVersion,
	currentUnityVersion,
	isVRCProject,
	mayUseChinaVariant = false,
	projectPath,
	navigate,
}: {
	version: string;
	currentUnityVersion: string;
	isVRCProject: boolean;
	mayUseChinaVariant?: boolean;
	projectPath: string;
	navigate: NavigateFn;
}) {
	try {
		const data = detectChangeUnityKind(
			currentUnityVersion,
			targetUnityVersion,
			isVRCProject,
		);

		if (await commands.projectIsUnityLaunching(projectPath)) {
			toastError(tt("projects:toast:close unity before migration"));
			return;
		}
		const header = headerText(data);

		const unityVersions = await commands.environmentUnityVersions();
		const findResult = findUnityForUnityChange(
			unityVersions,
			targetUnityVersion,
			mayUseChinaVariant,
		);
		if (!findResult.found) {
			await openSingleDialog(NoExactUnity2022Dialog, {
				expectedVersion: targetUnityVersion,
				installWithUnityHubLink: findResult.installLink,
				header,
			});
			return;
		}

		using dialog = showDialog();

		let backupTypePromise: Promise<ProjectBackupType | null>;
		if (data.isVRC && data.isTargetVersionSupportedByVRC) {
			// for supported migrations, show dialog same as migration
			switch (data.kind) {
				case "upgradePatchOrMinor":
					backupTypePromise = dialog.ask(MigrationConfirmMigrationPatchDialog, {
						header,
						unity: targetUnityVersion,
					});
					break;
				case "upgradeMajor":
					backupTypePromise = dialog.ask(MigrationConfirmMigrationDialog, {
						header,
					});
			}
		}
		backupTypePromise ??= dialog.ask(UnityVersionChange, {
			data,
			header,
		});
		const backupType = await backupTypePromise;
		if (backupType == null) return;

		let unityPath: string;
		if (findResult.installations.length === 1) {
			unityPath = findResult.installations[0][0];
		} else {
			const selected = await dialog.ask(UnitySelectorDialog, {
				unityVersions: findResult.installations,
			});
			if (selected == null) return;
			unityPath = selected.unityPath;
		}

		let migrateProjectPath: string;
		switch (backupType) {
			case "none":
				migrateProjectPath = projectPath;
				break;
			case "copy": {
				migrateProjectPath = await dialog.ask(MigrationCopyingDialog, {
					projectPath,
					header,
				});
				break;
			}
			case "backupArchive": {
				const result = await dialog.ask(BackupProjectDialog, {
					projectPath,
					header,
				});
				if (result === "cancelled") return;
				migrateProjectPath = projectPath;
				break;
			}
			default:
				assertNever(backupType);
		}
		dialog.replace(<MigrationMigratingDialog header={header} />);

		if (
			data.isVRC &&
			data.kind === "upgradeMajor" &&
			targetUnityVersion.startsWith("2022.")
		) {
			await commands.projectMigrateProjectTo2022(migrateProjectPath);
		}

		const finalizeResult = await dialog.askClosing(
			MigrationCallingUnityForMigrationDialog,
			{
				unityPath,
				migrateProjectPath,
				header,
			},
		);

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
		await Promise.all([
			queryClient.invalidateQueries({
				queryKey: ["projectDetails", projectPath],
			}),
			queryClient.invalidateQueries({
				queryKey: ["environmentProjects"],
			}),
		]);
		if (migrateProjectPath !== projectPath) {
			await navigate({
				replace: true,
				to: "/projects/manage",
				search: { projectPath: migrateProjectPath },
			});
		}
	} catch (e) {
		console.error(e);
		toastThrownError(e);
	}
}

function headerText(data: ChangeUnityData) {
	if (data.isVRC && data.isTargetVersionSupportedByVRC) {
		switch (data.kind) {
			case "upgradePatchOrMinor":
			case "upgradeMajor":
				return tc("projects:manage:dialog:unity migrate header");
		}
	}

	return tc("projects:manage:dialog:unity change version header");
}

function NoExactUnity2022Dialog({
	expectedVersion,
	installWithUnityHubLink,
	dialog,
	header,
}: {
	expectedVersion: string;
	installWithUnityHubLink?: string;
	dialog: DialogContext<void>;
	header: React.ReactNode;
}) {
	return (
		<>
			<DialogTitle>{header}</DialogTitle>
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
					<Button
						onClick={() => void commands.utilOpenUrl(installWithUnityHubLink)}
					>
						{tc("projects:dialog:open unity hub")}
					</Button>
				)}
				<Button onClick={() => dialog.close()} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</>
	);
}

function MigrationConfirmMigrationPatchDialog({
	unity,
	dialog,
	header,
}: {
	unity: string;
	dialog: DialogContext<ProjectBackupType | null>;
	header: React.ReactNode;
}) {
	return (
		<>
			<DialogTitle>{header}</DialogTitle>
			<DialogDescription>
				<p className={"text-destructive"}>
					{tc("projects:dialog:migrate unity2022 patch description", { unity })}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close(null)} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close("none")} variant={"destructive"}>
					{tc("projects:button:migrate in-place")}
				</Button>
			</DialogFooter>
		</>
	);
}

function MigrationConfirmMigrationDialog({
	dialog,
	header,
}: {
	dialog: DialogContext<ProjectBackupType | null>;
	header: React.ReactNode;
}) {
	return (
		<>
			<DialogTitle>{header}</DialogTitle>
			<DialogDescription>
				<p>{tc("projects:dialog:vpm migrate description")}</p>
			</DialogDescription>
			<DialogFooter className={"gap-1"}>
				<Button onClick={() => dialog.close(null)}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close("backupArchive")}>
					{tc("projects:button:backup and migrate")}
				</Button>
				<Button onClick={() => dialog.close("copy")}>
					{tc("projects:button:migrate copy")}
				</Button>
				<Button onClick={() => dialog.close("none")} variant={"destructive"}>
					{tc("projects:button:migrate in-place")}
				</Button>
			</DialogFooter>
		</>
	);
}

function UnityVersionChange({
	data,
	dialog,
	header,
}: {
	data: ChangeUnityData;
	dialog: DialogContext<ProjectBackupType | null>;
	header: React.ReactNode;
}) {
	let mainMessage: React.ReactNode;

	if (data.kind === "changeChina") {
		mainMessage = tc("projects:manage:dialog:changing china releases");
	} else {
		const category = {
			downgradeMajor: "downgrade major",
			downgradePatchOrMinor: "downgrade minor",
			upgradePatchOrMinor: "upgrade minor",
			upgradeMajor: "upgrade major",
		}[data.kind];
		if (data.isVRC) {
			if (data.isTargetVersionSupportedByVRC) {
				mainMessage = tc([
					`projects:manage:dialog:${category} vrchat supported`,
					`projects:manage:dialog:${category}`,
				]);
			} else {
				mainMessage = tc([
					`projects:manage:dialog:${category} vrchat unsupported`,
					`projects:manage:dialog:${category}`,
				]);
			}
		} else {
			mainMessage = tc(`projects:manage:dialog:${category}`);
		}
	}

	return (
		<>
			<DialogTitle>{header}</DialogTitle>
			<DialogDescription>
				<p className={"text-destructive"}>{mainMessage}</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close(null)} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close("none")} variant={"destructive"}>
					{tc("projects:button:change unity version")}
				</Button>
			</DialogFooter>
		</>
	);
}

export function MigrationCopyingDialog({
	projectPath,
	dialog,
	header,
}: {
	projectPath: string;
	dialog: DialogContext<string>;
	header: React.ReactNode;
}) {
	const [progress, setProgress] = useState<TauriCopyProjectProgress>({
		proceed: 0,
		total: 1,
		last_proceed: "Collecting files...",
	});

	useEffect(() => {
		const [_, promise] = callAsyncCommand(
			commands.environmentCopyProjectForMigration,
			[projectPath],
			(progress) => {
				setProgress((prev) => {
					if (prev.proceed > progress.proceed) return prev;
					return progress;
				});
			},
		);

		promise.then(dialog.close, dialog.error);
	}, [projectPath, dialog.close, dialog.error]);

	return (
		<>
			<DialogTitle>{header}</DialogTitle>
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
		</>
	);
}

function MigrationMigratingDialog({ header }: { header: React.ReactNode }) {
	return (
		<>
			<DialogTitle>{header}</DialogTitle>
			<DialogDescription>
				<p>{tc("projects:migrating...")}</p>
				<p>{tc("projects:do not close")}</p>
			</DialogDescription>
		</>
	);
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////

type UnityInstallation = [path: string, version: string, fromHub: boolean];

type FindUnityResult = FindUnityFoundResult | FindUnityNotFoundResult;

type ProjectBackupType = "none" | "copy" | "backupArchive";

interface FindUnityFoundResult {
	found: true;
	installations: UnityInstallation[];
}

interface FindUnityNotFoundResult {
	installLink?: string;
	found: false;
}

type ChangeUnityKind =
	| "changeChina" // Changing between 'c' releases and non 'c' releases
	| "downgradeMajor"
	| "downgradePatchOrMinor"
	| "upgradePatchOrMinor"
	| "upgradeMajor";

type ChangeUnityData =
	| {
			kind: ChangeUnityKind;
			isVRC: false;
	  }
	| {
			kind: ChangeUnityKind;
			isVRC: true;
			isTargetVersionSupportedByVRC: boolean;
	  };

function detectChangeUnityKind(
	currentVersion: string,
	targetUnityVersion: string,
	isVRCProject: boolean,
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
		};
	} else {
		return {
			kind,
			isVRC: false,
		};
	}
}

function findUnityForUnityChange(
	unityVersions: TauriUnityVersions,
	targetUnityVersion: string,
	mayUseChinaVariant: boolean,
): FindUnityResult {
	let foundVersions = unityVersions.unity_paths.filter(
		([_p, v, _]) => v === targetUnityVersion,
	);
	// if international version not found, try to find china version
	if (
		foundVersions.length === 0 &&
		mayUseChinaVariant &&
		parseUnityVersion(targetUnityVersion)?.chinaIncrement == null
	) {
		const chinaVersion = `${targetUnityVersion}c1`;
		foundVersions = unityVersions.unity_paths.filter(
			([_p, v, _]) => v === chinaVersion,
		);
	}
	if (foundVersions.length === 0) {
		if (
			compareUnityVersionString(
				targetUnityVersion,
				unityVersions.recommended_version,
			) === 0
		) {
			return {
				// This is using link to international version but china version of hub will handle international to china conversion
				installLink: unityVersions.install_recommended_version_link,
				found: false,
			};
		} else {
			return {
				found: false,
			};
		}
	}
	return {
		found: true,
		installations: foundVersions,
	};
}

function MigrationCallingUnityForMigrationDialog({
	unityPath,
	migrateProjectPath,
	dialog,
	header,
}: {
	unityPath: string;
	migrateProjectPath: string;
	dialog: DialogContext<"cancelled" | TauriCallUnityForMigrationResult>;
	header: React.ReactNode;
}) {
	const [lines, setLines] = useState<[number, string][]>([]);

	useEffect(() => {
		let lineNumber = 0;
		const [, promise] = callAsyncCommand(
			commands.projectCallUnityForMigration,
			[migrateProjectPath, unityPath],
			(lineString) => {
				setLines((prev) => {
					lineNumber++;
					const line: [number, string] = [lineNumber, lineString];
					if (prev.length > 200) {
						return [...prev.slice(1), line];
					} else {
						return [...prev, line];
					}
				});
			},
		);

		promise.then(dialog.close, dialog.error);
	}, [migrateProjectPath, unityPath, dialog]);

	const ref = React.useRef<HTMLDivElement>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to scroll to bottom on lines changed
	React.useEffect(() => {
		ref.current?.scrollIntoView({ behavior: "auto" });
	}, [lines]);

	return (
		<>
			<DialogTitle>{header}</DialogTitle>
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
		</>
	);
}
