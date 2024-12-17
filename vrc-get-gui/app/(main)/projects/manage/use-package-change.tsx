import { ExternalLink } from "@/components/ExternalLink";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
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
import { isHandleable } from "@/lib/errors";
import { tc, tt } from "@/lib/i18n";
import { toastInfo, toastSuccess, toastThrownError } from "@/lib/toast";
import { compareVersion, toVersionString } from "@/lib/version";
import { CircleAlert } from "lucide-react";
import type React from "react";
import { useCallback, useMemo, useState } from "react";
import type { PackageRowInfo } from "./collect-package-row-info";

export type RequestedOperation =
	| {
			type: "install";
			pkg: TauriPackage;
			hasUnityIncompatibleLatest?: boolean;
	  }
	| {
			type: "upgradeAll";
			hasUnityIncompatibleLatest: boolean;
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
	  }
	| {
			type: "bulkInstalled";
			hasUnityIncompatibleLatest: boolean;
	  }
	| {
			type: "bulkReinstalled";
	  }
	| {
			type: "bulkRemoved";
	  };

type InstallStatus =
	| {
			status: "normal";
	  }
	| {
			status: "creatingChanges";
	  }
	| {
			status: "missing-dependencies";
			dependencies: string[];
	  }
	| {
			status: "promptingChanges";
			changes: TauriPendingProjectChanges;
			requested: RequestedOperation;
	  }
	| {
			status: "applyingChanges";
	  };

interface PackageChangeDialog {
	createChanges: (
		operation: RequestedOperation,
		createPromise: Promise<TauriPendingProjectChanges>,
	) => void;
	dialog: React.ReactNode;
	installingPackage: boolean;
}

export function usePackageChangeDialog({
	projectPath,
	onRefreshProject,
	packageRowsData,
	existingPackages,
}: {
	projectPath: string;
	onRefreshProject: () => void;
	packageRowsData: PackageRowInfo[];
	existingPackages?: [string, TauriBasePackageInfo][];
}): PackageChangeDialog {
	const [installStatus, setInstallStatus] = useState<InstallStatus>({
		status: "normal",
	});

	const createChanges = useCallback(
		async (
			operation: RequestedOperation,
			createPromise: Promise<TauriPendingProjectChanges>,
		) => {
			try {
				setInstallStatus({ status: "creatingChanges" });
				const changes = await createPromise;
				setInstallStatus({
					status: "promptingChanges",
					changes,
					requested: operation,
				});
			} catch (e) {
				if (isHandleable(e) && e.body.type === "MissingDependencies") {
					setInstallStatus({
						status: "missing-dependencies",
						dependencies: e.body.dependencies,
					});
				} else {
					console.error(e);
					toastThrownError(e);
					setInstallStatus({ status: "normal" });
				}
			}
		},
		[],
	);

	let dialogForState: React.ReactNode = null;

	switch (installStatus.status) {
		case "promptingChanges": {
			const applyChanges = async ({
				changes,
				requested,
			}: {
				changes: TauriPendingProjectChanges;
				requested: RequestedOperation;
			}) => {
				try {
					setInstallStatus({ status: "applyingChanges" });
					await commands.projectApplyPendingChanges(
						projectPath,
						changes.changes_version,
					);
					setInstallStatus({ status: "normal" });
					onRefreshProject();

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
							toastSuccess(
								tt("projects:manage:toast:all packages reinstalled"),
							);
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
							toastSuccess(
								tt("projects:manage:toast:selected packages installed"),
							);
							if (requested.hasUnityIncompatibleLatest) {
								toastInfo(
									tt(
										"projects:manage:toast:some package has newer latest with incompatible unity",
									),
								);
							}
							break;
						case "bulkRemoved":
							toastSuccess(
								tt("projects:manage:toast:selected packages removed"),
							);
							break;
						case "bulkReinstalled":
							toastSuccess(
								tt("projects:manage:toast:selected packages reinstalled"),
							);
							break;
						default:
							assertNever(requested);
					}
				} catch (e) {
					console.error(e);
					setInstallStatus({ status: "normal" });
					toastThrownError(e);
				}
			};

			const cancel = async () => {
				setInstallStatus({ status: "normal" });
				try {
					await commands.projectClearPendingChanges();
				} catch (e) {
					console.error(e);
					toastThrownError(e);
				}
			};

			dialogForState = (
				<ProjectChangesDialog
					packages={packageRowsData}
					changes={installStatus.changes}
					existingPackages={existingPackages}
					cancel={cancel}
					apply={() => applyChanges(installStatus)}
				/>
			);
			break;
		}
		case "missing-dependencies": {
			dialogForState = (
				<MissingDependenciesDialog
					dependencies={installStatus.dependencies}
					onClose={() => setInstallStatus({ status: "normal" })}
				/>
			);
			break;
		}
	}

	return {
		dialog: dialogForState,
		createChanges,
		installingPackage: installStatus.status !== "normal",
	};
}

function ProjectChangesDialog({
	changes,
	packages,
	existingPackages,
	cancel,
	apply,
}: {
	changes: TauriPendingProjectChanges;
	packages: PackageRowInfo[];
	existingPackages?: [string, TauriBasePackageInfo][];
	cancel: () => void;
	apply: () => void;
}) {
	const versionConflicts = changes.conflicts.filter(
		([_, c]) => c.packages.length > 0,
	);
	const unityConflicts = changes.conflicts.filter(([_, c]) => c.unity_conflict);
	const unlockedConflicts = changes.conflicts.flatMap(
		([_, c]) => c.unlocked_names,
	);

	const getPackageDisplayName = useMemo(() => {
		const packagesById = new Map(packages.map((p) => [p.id, p]));
		return (pkgId: string) => packagesById.get(pkgId)?.displayName ?? pkgId;
	}, [packages]);

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

	const ChangelogButton = ({ url }: { url?: string | null }) => {
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
	};

	return (
		<DialogOpen className={"whitespace-normal"}>
			<DialogTitle>{tc("projects:manage:button:apply changes")}</DialogTitle>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"overflow-y-auto max-h-[50vh]"}>
				<p>{tc("projects:manage:dialog:confirm changes description")}</p>
				<div className={"flex flex-col gap-1 p-2"}>
					{installingNewPackages.map(([pkgId, pkgChange]) => {
						const name =
							pkgChange.InstallNew.display_name ?? pkgChange.InstallNew.name;
						const version = toVersionString(pkgChange.InstallNew.version);

						return (
							<div key={pkgId} className={"flex items-center p-3"}>
								<p className={"font-normal"}>
									{tc("projects:manage:dialog:install package", {
										name,
										version,
									})}
								</p>
								<ChangelogButton url={pkgChange.InstallNew.changelog_url} />
							</div>
						);
					})}
					{installingNewPackages.length > 0 &&
						reInstallingPackages.length > 0 && <hr />}
					{reInstallingPackages.map(([pkgId, pkgChange]) => {
						const name =
							pkgChange.InstallNew.display_name ?? pkgChange.InstallNew.name;
						const version = toVersionString(pkgChange.InstallNew.version);

						return (
							<div key={pkgId} className={"flex items-center p-3"}>
								<p className={"font-normal"}>
									{tc("projects:manage:dialog:reinstall package", {
										name,
										version,
									})}
								</p>
								<ChangelogButton url={pkgChange.InstallNew.changelog_url} />
							</div>
						);
					})}
					{removingRequestedPackages.map(([pkgId, _]) => {
						const name = getPackageDisplayName(pkgId);
						return (
							<TypographyItem key={pkgId}>
								{tc("projects:manage:dialog:uninstall package as requested", {
									name,
								})}
							</TypographyItem>
						);
					})}
					{removingLegacyPackages.map(([pkgId, _]) => {
						const name = getPackageDisplayName(pkgId);
						return (
							<TypographyItem key={pkgId}>
								{tc("projects:manage:dialog:uninstall package as legacy", {
									name,
								})}
							</TypographyItem>
						);
					})}
					{removingUnusedPackages.map(([pkgId, _]) => {
						const name = getPackageDisplayName(pkgId);
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
										{ pkg: getPackageDisplayName(pkgId) },
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
				<Button onClick={cancel} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={apply} variant={"destructive"}>
					{tc("projects:manage:button:apply")}
				</Button>
			</DialogFooter>
		</DialogOpen>
	);
}

function comparePackageChangeByName(
	[aName, _1]: [string, TauriPackageChange],
	[bName, _2]: [string, TauriPackageChange],
): number {
	return aName.localeCompare(bName);
}

function MissingDependenciesDialog({
	dependencies,
	onClose,
}: { dependencies: string[]; onClose: () => void }) {
	return (
		<DialogOpen>
			<DialogTitle className={"text-destructive"}>
				<CircleAlert className="size-6 inline" />{" "}
				{tc("projects:manage:dialog:missing dependencies")}
			</DialogTitle>
			<DialogDescription>
				<p className={"whitespace-normal"}>
					{tc("projects:manage:dialog:missing dependencies description")}
				</p>
				<ul className={"list-disc ml-4"}>
					{dependencies.map((dep) => (
						<li key={dep}>{dep}</li>
					))}
				</ul>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={onClose}>{tc("general:button:close")}</Button>
			</DialogFooter>
		</DialogOpen>
	);
}
