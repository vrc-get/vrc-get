import { ExternalLink } from "@/components/ExternalLink";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriBasePackageInfo,
	TauriPackage,
	TauriPackageChange,
	TauriPendingProjectChanges,
	TauriRemoveReason,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { isHandleable } from "@/lib/errors";
import { tc, tt } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastInfo, toastSuccess, toastThrownError } from "@/lib/toast";
import { compareVersion, toVersionString } from "@/lib/version";
import type { DefaultError } from "@tanstack/query-core";
import { type UseMutationOptions, queryOptions } from "@tanstack/react-query";
import { CircleAlert } from "lucide-react";
import type React from "react";

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
			return commands.projectInstallPackages(
				projectPath,
				operation.pkg.env_version,
				[operation.pkg.index],
			);
		case "upgradeAll":
			return commands.projectInstallPackages(
				projectPath,
				...packagesToIndexes(operation.packages),
			);
		case "resolve":
		case "reinstallAll":
			return commands.projectResolve(projectPath);
		case "remove":
			return commands.projectRemovePackages(projectPath, [operation.packageId]);
		case "bulkInstalled":
			return commands.projectInstallPackages(
				projectPath,
				...packagesToIndexes(operation.packages),
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

function packagesToIndexes(
	packages: TauriPackage[],
): [envVersion: number, packagesIndexes: number[]] {
	let envVersion: number | undefined = undefined;
	const packagesIndexes: number[] = [];
	for (const pkg of packages) {
		if (envVersion == null) envVersion = pkg.env_version;
		else if (envVersion !== pkg.env_version)
			throw new Error("Inconsistent env_version");

		packagesIndexes.push(pkg.index);
	}
	if (envVersion == null) {
		throw new Error("projects:manage:toast:no upgradable");
	}
	return [envVersion, packagesIndexes];
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

	function isInstallNew(
		pair: [string, TauriPackageChange],
	): pair is [string, { InstallNew: TauriPackage }] {
		return "InstallNew" in pair[1];
	}

	function isRemove(
		pair: [string, TauriPackageChange],
	): pair is [string, { Remove: TauriRemoveReason }] {
		return "Remove" in pair[1];
	}

	const existingPackageMap = new Map(existingPackages ?? []);

	const installingPackages = changes.package_changes.filter(isInstallNew);
	const removingPackages = changes.package_changes.filter(isRemove);

	const installingPackageById = new Map(installingPackages);

	const reInstallingPackages = installingPackages.filter(([pkgId, c]) => {
		const info = existingPackageMap.get(pkgId);
		return (
			info !== undefined &&
			compareVersion(c.InstallNew.version, info.version) === 0
		);
	});
	const installingNewPackages = installingPackages.filter(([pkgId, c]) => {
		const info = existingPackageMap.get(pkgId);
		return (
			info === undefined ||
			compareVersion(c.InstallNew.version, info.version) !== 0
		);
	});

	const removingRequestedPackages = removingPackages.filter(
		([_, c]) => c.Remove === "Requested",
	);
	const removingLegacyPackages = removingPackages.filter(
		([_, c]) => c.Remove === "Legacy",
	);
	const removingUnusedPackages = removingPackages.filter(
		([_, c]) => c.Remove === "Unused",
	);

	reInstallingPackages.sort(comparePackageChangeByName);
	installingNewPackages.sort(comparePackageChangeByName);
	removingRequestedPackages.sort(comparePackageChangeByName);
	removingLegacyPackages.sort(comparePackageChangeByName);
	removingUnusedPackages.sort(comparePackageChangeByName);

	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:manage:button:apply changes")}</DialogTitle>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"overflow-y-auto max-h-[50vh]"}>
				<p>{tc("projects:manage:dialog:confirm changes description")}</p>
				<div className={"flex flex-col gap-1 p-2"}>
					{installingNewPackages.map(([pkgId, pkgChange]) => {
						return (
							<InstallPackageInfo
								key={pkgId}
								pkgChange={pkgChange}
								message={"projects:manage:dialog:install package"}
							/>
						);
					})}
					{installingNewPackages.length > 0 &&
						reInstallingPackages.length > 0 && <hr />}
					{reInstallingPackages.map(([pkgId, pkgChange]) => {
						return (
							<InstallPackageInfo
								key={pkgId}
								pkgChange={pkgChange}
								message={"projects:manage:dialog:reinstall package"}
							/>
						);
					})}
					{removingRequestedPackages.map(([pkgId, _]) => {
						const name = existingPackageMap.get(pkgId)?.display_name ?? pkgId;
						return (
							<TypographyItem key={pkgId}>
								{tc("projects:manage:dialog:uninstall package as requested", {
									name,
								})}
							</TypographyItem>
						);
					})}
					{removingLegacyPackages.map(([pkgId, _]) => {
						const name = existingPackageMap.get(pkgId)?.display_name ?? pkgId;
						return (
							<TypographyItem key={pkgId}>
								{tc("projects:manage:dialog:uninstall package as legacy", {
									name,
								})}
							</TypographyItem>
						);
					})}
					{removingUnusedPackages.map(([pkgId, _]) => {
						const name = existingPackageMap.get(pkgId)?.display_name ?? pkgId;
						return (
							<TypographyItem key={pkgId}>
								{tc("projects:manage:dialog:uninstall package as unused", {
									name,
								})}
							</TypographyItem>
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
								function getPackageDisplayName(id: string) {
									return (
										installingPackageById.get(id)?.InstallNew?.display_name ??
										existingPackageMap.get(id)?.display_name ??
										pkgId
									);
								}
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
											pkg:
												installingPackageById.get(pkgId)?.InstallNew
													?.display_name ?? pkgId,
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
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close(false)} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close(true)} variant={"destructive"}>
					{tc("projects:manage:button:apply")}
				</Button>
			</DialogFooter>
		</div>
	);
}

function InstallPackageInfo({
	pkgChange,
	message,
}: {
	pkgChange: { InstallNew: TauriBasePackageInfo };
	message: string;
}) {
	const name = pkgChange.InstallNew.display_name ?? pkgChange.InstallNew.name;
	const version = toVersionString(pkgChange.InstallNew.version);

	return (
		<div className={"flex items-center p-3"}>
			<p className={"font-normal"}>
				{tc(message, {
					name,
					version,
				})}
			</p>
			<ChangelogButton url={pkgChange.InstallNew.changelog_url} />
		</div>
	);
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
}: { dependencies: string[]; dialog: DialogContext<void> }) {
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
					{dependencies.map((dep) => (
						<li key={dep}>{dep}</li>
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
