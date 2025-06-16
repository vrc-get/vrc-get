import Loading from "@/app/-loading";
import { DelayedButton } from "@/components/DelayedButton";
import { ExternalLink } from "@/components/ExternalLink";
import { Button } from "@/components/ui/button";
import {
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriBasePackageInfo,
	TauriPackage,
	TauriPackageChange,
	TauriPendingProjectChanges,
	TauriVersion,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { isHandleable } from "@/lib/errors";
import { tc, tt } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastInfo, toastSuccess, toastThrownError } from "@/lib/toast";
import { groupBy, keyComparator } from "@/lib/utils";
import { compareVersion, toVersionString } from "@/lib/version";
import type { DefaultError } from "@tanstack/query-core";
import { type UseMutationOptions, queryOptions } from "@tanstack/react-query";
import { CircleAlert } from "lucide-react";
import React from "react";
import { Fragment } from "react";

export type RequestedOperation =
	| {
			type: "install";
			pkg: TauriPackage;
			hasUnityIncompatibleLatest?: boolean;
	  }
	| {
			type: "upgradeAll";
			hasUnityIncompatibleLatest: boolean;
			packages: TauriPackage[];
	  }
	| {
			type: "resolve";
	  }
	| {
			type: "reinstallAll";
	  }
	| {
			type: "remove";
			displayName: string;
			packageId: string;
	  }
	| {
			type: "bulkInstalled";
			hasUnityIncompatibleLatest: boolean;
			packages: TauriPackage[];
	  }
	| {
			type: "bulkReinstalled";
			packageIds: string[];
	  }
	| {
			type: "bulkRemoved";
			packageIds: string[];
	  };

function environmentPackages(projectPath: string) {
	return queryOptions({
		queryKey: ["projectDetails", projectPath],
		queryFn: () => commands.projectDetails(projectPath),
		refetchOnWindowFocus: false,
	});
}

function mutationOptions<
	TOptions extends UseMutationOptions<TData, TError, TVariables, TContext>,
	TData = unknown,
	TError = DefaultError,
	TVariables = void,
	TContext = unknown,
>(
	options: TOptions & UseMutationOptions<TData, TError, TVariables, TContext>,
): TOptions {
	return options;
}

export function applyChangesMutation(projectPath: string) {
	return mutationOptions({
		mutationKey: ["projectApplyChanges", projectPath],
		mutationFn: async (operation: RequestedOperation) =>
			await applyChanges(projectPath, operation),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["projectDetails", projectPath],
			});
			await queryClient.invalidateQueries({
				queryKey: ["environmentPackages"],
			});
		},
	});
}

export async function applyChanges(
	projectPath: string,
	operation: RequestedOperation,
) {
	try {
		const existingPackages = queryClient.getQueryData(
			environmentPackages(projectPath).queryKey,
		)?.installed_packages;

		const changes = await createChanges(projectPath, operation);
		if (
			!(await openSingleDialog(ProjectChangesDialog, {
				changes,
				existingPackages,
			}))
		) {
			// close window
			return;
		}
		await commands.projectApplyPendingChanges(
			projectPath,
			changes.changes_version,
		);
		showToast(operation);
	} catch (e) {
		if (isHandleable(e) && e.body.type === "MissingDependencies") {
			await openSingleDialog(MissingDependenciesDialog, {
				dependencies: e.body.dependencies,
			});
		} else {
			throw e;
		}
	}
}

function createChanges(
	projectPath: string,
	operation: RequestedOperation,
): Promise<TauriPendingProjectChanges> {
	switch (operation.type) {
		case "install":
			return commands.projectInstallPackages(projectPath, [
				[operation.pkg.name, toVersionString(operation.pkg.version)],
			]);
		case "upgradeAll":
			return commands.projectInstallPackages(
				projectPath,
				operation.packages.map((pkg) => [
					pkg.name,
					toVersionString(pkg.version),
				]),
			);
		case "resolve":
		case "reinstallAll":
			return commands.projectResolve(projectPath);
		case "remove":
			return commands.projectRemovePackages(projectPath, [operation.packageId]);
		case "bulkInstalled":
			return commands.projectInstallPackages(
				projectPath,
				operation.packages.map((pkg) => [
					pkg.name,
					toVersionString(pkg.version),
				]),
			);
		case "bulkReinstalled":
			return commands.projectReinstallPackages(
				projectPath,
				operation.packageIds,
			);
		case "bulkRemoved":
			return commands.projectRemovePackages(projectPath, operation.packageIds);
		default:
			assertNever(operation);
	}
}
function showToast(requested: RequestedOperation) {
	switch (requested.type) {
		case "install":
			toastSuccess(
				tt("projects:manage:toast:package installed", {
					name: requested.pkg.display_name ?? requested.pkg.name,
					version: toVersionString(requested.pkg.version),
				}),
			);
			if (requested.hasUnityIncompatibleLatest) {
				toastInfo(
					tt(
						"projects:manage:toast:the package has newer latest with incompatible unity",
					),
				);
			}
			break;
		case "remove":
			toastSuccess(
				tt("projects:manage:toast:package removed", {
					name: requested.displayName,
				}),
			);
			break;
		case "resolve":
			toastSuccess(tt("projects:manage:toast:resolved"));
			break;
		case "reinstallAll":
			toastSuccess(tt("projects:manage:toast:all packages reinstalled"));
			break;
		case "upgradeAll":
			toastSuccess(tt("projects:manage:toast:all packages upgraded"));
			if (requested.hasUnityIncompatibleLatest) {
				toastInfo(
					tt(
						"projects:manage:toast:some package has newer latest with incompatible unity",
					),
				);
			}
			break;
		case "bulkInstalled":
			toastSuccess(tt("projects:manage:toast:selected packages installed"));
			if (requested.hasUnityIncompatibleLatest) {
				toastInfo(
					tt(
						"projects:manage:toast:some package has newer latest with incompatible unity",
					),
				);
			}
			break;
		case "bulkRemoved":
			toastSuccess(tt("projects:manage:toast:selected packages removed"));
			break;
		case "bulkReinstalled":
			toastSuccess(tt("projects:manage:toast:selected packages reinstalled"));
			break;
		default:
			assertNever(requested);
	}
}

function ProjectChangesDialog({
	changes,
	existingPackages,
	dialog,
}: {
	changes: TauriPendingProjectChanges;
	existingPackages?: [string, TauriBasePackageInfo][];
	dialog: DialogContext<boolean>;
}) {
	const versionConflicts = changes.conflicts.filter(
		([_, c]) => c.packages.length > 0,
	);
	const unityConflicts = changes.conflicts.filter(([_, c]) => c.unity_conflict);
	const unlockedConflicts = changes.conflicts.flatMap(
		([_, c]) => c.unlocked_names,
	);

	const TypographyItem = ({ children }: { children: React.ReactNode }) => (
		<div className={"p-3"}>
			<p className={"font-normal"}>{children}</p>
		</div>
	);

	const existingPackageMap = new Map(existingPackages ?? []);

	const categorizedChanges = changes.package_changes.map(([pkgId, change]) =>
		categorizeChange(pkgId, change, existingPackageMap),
	);
	categorizedChanges.sort(keyComparator("packageId"));
	const groupedChanges = Array.from(groupBy(categorizedChanges, (c) => c.type));
	groupedChanges.sort(keyComparator(0));

	const installingPackageById = new Map(
		changes.package_changes
			.map(([id, change]) =>
				"InstallNew" in change ? ([id, change.InstallNew] as const) : undefined,
			)
			.filter((x) => x != null),
	);

	function getPackageDisplayName(id: string) {
		return (
			installingPackageById.get(id)?.display_name ??
			existingPackageMap.get(id)?.display_name ??
			id
		);
	}

	const breakingChanges = groupedChanges.some(
		([a]) => a === PackageChangeCategory.UpgradeMajor,
	);

	const incompatibility = changes.conflicts.length !== 0;

	const needsCare = breakingChanges || incompatibility;

	return (
		<div className={"contents whitespace-normal"}>
			<DialogHeader>
				<DialogTitle>{tc("projects:manage:button:apply changes")}</DialogTitle>
				<DialogDescription>
					<p>{tc("projects:manage:dialog:confirm changes description")}</p>
					{breakingChanges && (
						<div
							className={
								"flex border border-solid border-warning mt-3 py-2 me-1.5"
							}
						>
							<CircleAlert
								className={"text-warning self-center mx-2 shrink-0"}
							/>
							<p>{tc("projects:manage:dialog:note breaking changes")}</p>
						</div>
					)}
					{incompatibility && (
						<div
							className={
								"flex border border-solid border-warning mt-3 py-2 me-1.5"
							}
						>
							<CircleAlert
								className={"text-warning self-center mx-2 shrink-0"}
							/>
							<p>{tc("projects:manage:dialog:note incompatibility")}</p>
						</div>
					)}
				</DialogDescription>
			</DialogHeader>
			<div className="overflow-hidden flex">
				<ScrollArea
					type="always"
					className={"w-full"}
					scrollBarClassName={"bg-background pb-2.5"}
				>
					<div className="pr-2 overflow-x-hidden">
						<div className={"flex flex-col gap-1 p-2"}>
							{groupedChanges.map(([category, changes], index) => {
								return (
									<Fragment key={category}>
										{index !== 0 && <hr />}
										{changes.map((change) => (
											<PackageChange key={change.packageId} change={change} />
										))}
									</Fragment>
								);
							})}
						</div>
						{versionConflicts.length > 0 ? (
							<>
								<p className={"text-destructive"}>
									{tc("projects:manage:dialog:package version conflicts", {
										count: versionConflicts.length,
									})}
								</p>
								<div className={"flex flex-col gap-1 p-2"}>
									{versionConflicts.map(([pkgId, conflict]) => {
										return (
											<TypographyItem key={pkgId}>
												{tc("projects:manage:dialog:conflicts with", {
													pkg: getPackageDisplayName(pkgId),
													other: conflict.packages
														.map((p) => getPackageDisplayName(p))
														.join(", "),
												})}
											</TypographyItem>
										);
									})}
								</div>
							</>
						) : null}
						{unityConflicts.length > 0 ? (
							<>
								<p className={"text-destructive"}>
									{tc("projects:manage:dialog:unity version conflicts", {
										count: unityConflicts.length,
									})}
								</p>
								<div className={"flex flex-col gap-1 p-2"}>
									{unityConflicts.map(([pkgId, _]) => (
										<TypographyItem key={pkgId}>
											{tc(
												"projects:manage:dialog:package not supported your unity",
												{
													pkg: getPackageDisplayName(pkgId),
												},
											)}
										</TypographyItem>
									))}
								</div>
							</>
						) : null}
						{changes.remove_legacy_files.length > 0 ||
						changes.remove_legacy_folders.length > 0 ? (
							<>
								<p className={"text-destructive"}>
									{tc(
										"projects:manage:dialog:files and directories are removed as legacy",
									)}
								</p>
								<div className={"flex flex-col gap-1 p-2"}>
									{changes.remove_legacy_files.map((f) => (
										<TypographyItem key={f}>{f}</TypographyItem>
									))}
									{changes.remove_legacy_folders.map((f) => (
										<TypographyItem key={f}>{f}</TypographyItem>
									))}
								</div>
							</>
						) : null}
						{unlockedConflicts.length > 0 ? (
							<>
								<p className={"text-destructive"}>
									{tc(
										"projects:manage:dialog:packages installed in the following directories will be removed",
									)}
								</p>
								<div className={"flex flex-col gap-1 p-2"}>
									{unlockedConflicts.map((f) => (
										<TypographyItem key={f}>{f}</TypographyItem>
									))}
								</div>
							</>
						) : null}
					</div>
				</ScrollArea>
			</div>
			<DialogFooter>
				<Button onClick={() => dialog.close(false)} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<DelayedButton
					onClick={() => dialog.close(true)}
					variant={needsCare ? "destructive" : "warning"}
					delay={needsCare ? 1000 : 0}
				>
					{tc("projects:manage:button:apply")}
				</DelayedButton>
			</DialogFooter>
		</div>
	);
}

function PackageChange({
	change,
}: {
	change: PackageChangeDisplayInformation;
}) {
	switch (change.type) {
		case PackageChangeCategory.UpgradeMajor:
			return (
				<div className={"flex items-center p-3 justify-between bg-warning/10"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:upgrade package", {
							name: change.displayName,
							previousVersion: toVersionString(change.previousVersion),
							version: toVersionString(change.version),
						})}
						<span className={"text-warning"}>
							{"\u200B"}
							<CircleAlert
								className={
									"inline px-1 size-5 -mt-0.5 box-content align-middle"
								}
							/>
							{tc("projects:manage:dialog:breaking changes")}
						</span>
					</p>
					<ChangelogButton url={change.changelogUrl} />
				</div>
			);
		case PackageChangeCategory.Upgrade:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:upgrade package", {
							name: change.displayName,
							previousVersion: toVersionString(change.previousVersion),
							version: toVersionString(change.version),
						})}
					</p>
					<ChangelogButton url={change.changelogUrl} />
				</div>
			);
		case PackageChangeCategory.Downgrade:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:downgrade package", {
							name: change.displayName,
							previousVersion: toVersionString(change.previousVersion),
							version: toVersionString(change.version),
						})}
					</p>
					<ChangelogButton url={change.changelogUrl} />
				</div>
			);
		case PackageChangeCategory.InstallNew:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:install package", {
							name: change.displayName,
							version: toVersionString(change.version),
						})}
					</p>
					<ChangelogButton url={change.changelogUrl} />
				</div>
			);
		case PackageChangeCategory.UninstallRequested:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:uninstall package as requested", {
							name: change.displayName,
						})}
					</p>
				</div>
			);
		case PackageChangeCategory.UninstallUnused:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:uninstall package as unused", {
							name: change.displayName,
						})}
					</p>
				</div>
			);
		case PackageChangeCategory.UninstallLegacy:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal"}>
						{tc("projects:manage:dialog:uninstall package as legacy", {
							name: change.displayName,
						})}
					</p>
				</div>
			);
		case PackageChangeCategory.Reinstall:
			return (
				<div className={"flex items-center p-3 justify-between"}>
					<p className={"font-normal select-text"}>
						{tc("projects:manage:dialog:reinstall package", {
							name: change.displayName,
							version: toVersionString(change.version),
						})}
					</p>
					<ChangelogButton url={change.changelogUrl} />
				</div>
			);
	}
}

enum PackageChangeCategory {
	InstallNew = 0,
	UpgradeMajor = 1,
	Upgrade = 2,
	Downgrade = 3,
	UninstallRequested = 4,
	UninstallUnused = 5,
	UninstallLegacy = 6,
	Reinstall = 7,
}

type PackageChangeDisplayInformation = {
	packageId: string;
	displayName: string;
} & (
	| {
			type: PackageChangeCategory.UpgradeMajor;
			version: TauriVersion;
			previousVersion: TauriVersion;
			changelogUrl: string | null;
	  }
	| {
			type: PackageChangeCategory.Upgrade;
			version: TauriVersion;
			previousVersion: TauriVersion;
			changelogUrl: string | null;
	  }
	| {
			type: PackageChangeCategory.Downgrade;
			version: TauriVersion;
			previousVersion: TauriVersion;
			changelogUrl: string | null;
	  }
	| {
			type: PackageChangeCategory.Reinstall;
			version: TauriVersion;
			changelogUrl: string | null;
	  }
	| {
			type: PackageChangeCategory.InstallNew;
			version: TauriVersion;
			changelogUrl: string | null;
	  }
	| {
			type: PackageChangeCategory.UninstallRequested;
	  }
	| {
			type: PackageChangeCategory.UninstallUnused;
	  }
	| {
			type: PackageChangeCategory.UninstallLegacy;
	  }
);

function categorizeChange(
	pkgId: string,
	change: TauriPackageChange,
	installedPackages: Map<string, TauriBasePackageInfo>,
): PackageChangeDisplayInformation {
	if ("InstallNew" in change) {
		const name = change.InstallNew.display_name ?? change.InstallNew.name;

		const installed = installedPackages.get(pkgId);
		if (installed == null) {
			return {
				packageId: pkgId,
				displayName: name,
				type: PackageChangeCategory.InstallNew,
				version: change.InstallNew.version,
				changelogUrl: change.InstallNew.changelog_url,
			};
		} else {
			const compare = compareVersion(
				installed.version,
				change.InstallNew.version,
			);
			switch (compare) {
				case 1:
					return {
						packageId: pkgId,
						displayName: name,
						type: PackageChangeCategory.Downgrade,
						version: change.InstallNew.version,
						previousVersion: installed.version,
						changelogUrl: change.InstallNew.changelog_url,
					};
				case 0:
					return {
						packageId: pkgId,
						displayName: name,
						type: PackageChangeCategory.Reinstall,
						version: change.InstallNew.version,
						changelogUrl: change.InstallNew.changelog_url,
					};
				case -1:
					if (
						isUpgradingMajorly(
							pkgId,
							installed.version,
							change.InstallNew.version,
						)
					) {
						return {
							packageId: pkgId,
							displayName: name,
							type: PackageChangeCategory.UpgradeMajor,
							version: change.InstallNew.version,
							previousVersion: installed.version,
							changelogUrl: change.InstallNew.changelog_url,
						};
					} else {
						return {
							packageId: pkgId,
							displayName: name,
							type: PackageChangeCategory.Upgrade,
							version: change.InstallNew.version,
							previousVersion: installed.version,
							changelogUrl: change.InstallNew.changelog_url,
						};
					}
			}
		}
	} else {
		const name = installedPackages.get(pkgId)?.display_name ?? pkgId;
		switch (change.Remove) {
			case "Requested":
				return {
					packageId: pkgId,
					displayName: name,
					type: PackageChangeCategory.UninstallRequested,
				};
			case "Legacy":
				return {
					packageId: pkgId,
					displayName: name,
					type: PackageChangeCategory.UninstallLegacy,
				};
			case "Unused":
				return {
					packageId: pkgId,
					displayName: name,
					type: PackageChangeCategory.UninstallUnused,
				};
		}
	}
}

function isUpgradingMajorly(
	pkgId: string,
	prevVersion: TauriVersion,
	newVersion: TauriVersion,
): boolean {
	function firstNonZeroVersionNum(version: TauriVersion): number {
		if (version.major !== 0) return version.major;
		if (version.minor !== 0) return version.minor;
		return version.patch;
	}

	// generic case: non-zero first version number will be the major version
	if (
		firstNonZeroVersionNum(prevVersion) !== firstNonZeroVersionNum(newVersion)
	) {
		return true;
	}
	// Special case: VRChat SDK uses Branding.Breaking.Bumps.
	// Therefore the second number bump means major version bump.
	// See https://vcc.docs.vrchat.com/vpm/packages/#brandingbreakingbumps
	// See https://feedback.vrchat.com/sdk-bug-reports/p/feedback-please-dont-make-vrcsdk-to-4x-unless-as-big-breaking-changes-as-2-to-3
	if (
		pkgId === "com.vrchat.avatars" ||
		pkgId === "com.vrchat.worlds" ||
		pkgId === "com.vrchat.base"
	) {
		if (prevVersion.minor !== newVersion.minor) {
			return true;
		}
	}

	// No conditions met so it's not major bump
	return false;
}

function ChangelogButton({ url }: { url?: string | null }) {
	if (url == null) return null;
	try {
		const parsed = new URL(url);
		if (parsed.protocol === "http:" || parsed.protocol === "https:") {
			return (
				<Button
					className={"ml-1 px-2"}
					size={"sm"}
					onClick={() => commands.utilOpenUrl(url)}
				>
					<ExternalLink>
						{tc("projects:manage:button:see changelog")}
					</ExternalLink>
				</Button>
			);
		}
	} catch {}

	return null;
}

function comparePackageChangeByName(
	[aName]: [string, TauriPackageChange],
	[bName]: [string, TauriPackageChange],
): number {
	return aName.localeCompare(bName);
}

function MissingDependenciesDialog({
	dependencies,
	dialog,
}: {
	dependencies: [pkg: string, range: string][];
	dialog: DialogContext<void>;
}) {
	return (
		<div>
			<DialogTitle className={"text-destructive"}>
				<CircleAlert className="size-6 inline" />{" "}
				{tc("projects:manage:dialog:missing dependencies")}
			</DialogTitle>
			<DialogDescription>
				<p className={"whitespace-normal"}>
					{tc("projects:manage:dialog:missing dependencies description")}
				</p>
				<ul className={"list-disc ml-4 mt-2"}>
					{dependencies.map(([dep, range]) => (
						<li key={dep}>
							{dep} version {range}
						</li>
					))}
				</ul>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close()}>
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</div>
	);
}
