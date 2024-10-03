import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import i18next, { tc } from "@/lib/i18n";
import { toastError, toastNormal } from "@/lib/toast";
import { useUnitySelectorDialog } from "@/lib/use-unity-selector-dialog";
import { parseUnityVersion } from "@/lib/version";
import React from "react";

export type OpenUnityFunction = (
	projectPath: string,
	unityVersion: string | null,
	unityRevision?: string | null,
) => void;

export type Result = {
	dialog: React.ReactNode;
	openUnity: OpenUnityFunction;
};

type StateInternal =
	| {
			state: "normal";
	  }
	| {
			state: "ask-for-china-version";
			projectUnityVersion: string;
			chinaUnityVersion: string;
			projectPath: string;
	  }
	| {
			state: "ask-for-international-version";
			projectUnityVersion: string;
			internationalUnityVersion: string;
			projectPath: string;
	  }
	| {
			state: "suggest-unity-hub";
			unityVersion: string;
			unityHubLink: string;
	  };

export function useOpenUnity(): Result {
	const unitySelector = useUnitySelectorDialog();
	const [installStatus, setInstallStatus] = React.useState<StateInternal>({
		state: "normal",
	});

	const openUnity = async (
		projectPath: string,
		unityVersion: string | null,
		unityRevision?: string | null,
		freezeVersion?: boolean,
	) => {
		if (unityVersion == null) {
			toastError(i18next.t("projects:toast:invalid project unity version"));
			return;
		}
		let [unityVersions, selectedPath] = await Promise.all([
			commands.environmentUnityVersions(),
			commands.projectGetUnityPath(projectPath),
		]);
		if (unityVersions == null) {
			toastError(
				i18next.t("projects:toast:match version unity not found", {
					unity: unityVersion,
				}),
			);
			return;
		}

		let foundVersions = unityVersions.unity_paths.filter(
			([_p, v, _i]) => v === unityVersion,
		);

		if (foundVersions.length === 0) {
			if (await commands.environmentIsLoadingFromUnityHubInProgress()) {
				toastNormal(tc("projects:toast:loading unity from unity hub"));
				await commands.environmentWaitForUnityHubUpdate();
				unityVersions = await commands.environmentUnityVersions();
				foundVersions = unityVersions.unity_paths.filter(
					([_p, v, _i]) => v === unityVersion,
				);
			}
		}

		switch (foundVersions.length) {
			case 0: {
				if (!freezeVersion) {
					// if requested version is not china version and china version is available, suggest to use china version
					// if requested version is china version and international version is available, suggest to use international version
					if (parseUnityVersion(unityVersion)?.chinaIncrement == null) {
						// unityVersion is international version, find china version
						const chinaVersion = `${unityVersion}c1`;
						const hasChinaVersion = unityVersions.unity_paths.some(
							([_p, v, _i]) => v === chinaVersion,
						);
						if (hasChinaVersion) {
							setInstallStatus({
								state: "ask-for-china-version",
								projectUnityVersion: unityVersion,
								chinaUnityVersion: chinaVersion,
								projectPath,
							});
							return;
						}
					} else {
						// unityVersion is china version, find international version
						const internationalVersion = unityVersion.replace(/c\d+$/, "");
						const hasInternationalRevision = unityVersions.unity_paths.some(
							([_p, v, _i]) => v === internationalVersion,
						);
						if (hasInternationalRevision) {
							setInstallStatus({
								state: "ask-for-international-version",
								projectUnityVersion: unityVersion,
								internationalUnityVersion: internationalVersion,
								projectPath,
							});
							return;
						}
					}
				}
				if (unityRevision) {
					setInstallStatus({
						state: "suggest-unity-hub",
						unityVersion: unityVersion,
						unityHubLink: `unityhub://${unityVersion}/${unityRevision}`,
					});
				} else {
					toastError(
						i18next.t("projects:toast:match version unity not found", {
							unity: unityVersion,
						}),
					);
				}
				return;
			}
			case 1:
				{
					if (selectedPath) {
						if (foundVersions[0][0] !== selectedPath) {
							// if only unity is not
							void commands.projectSetUnityPath(projectPath, null);
						}
					}
					const result = await commands.projectOpenUnity(
						projectPath,
						foundVersions[0][0],
					);
					if (result) toastNormal(i18next.t("projects:toast:opening unity..."));
					else toastError(i18next.t("projects:toast:unity already running"));
				}
				return;
			default: {
				if (selectedPath) {
					const found = foundVersions.find(([p, _v, _i]) => p === selectedPath);
					if (found) {
						const result = await commands.projectOpenUnity(
							projectPath,
							selectedPath,
						);
						if (result)
							toastNormal(i18next.t("projects:toast:opening unity..."));
						else toastError(i18next.t("projects:toast:unity already running"));
						return;
					}
				}
				const selected = await unitySelector.select(foundVersions, true);
				if (selected == null) return;
				if (selected.keepUsingThisVersion) {
					void commands.projectSetUnityPath(projectPath, selected.unityPath);
				}
				const result = await commands.projectOpenUnity(
					projectPath,
					selected.unityPath,
				);
				if (result) toastNormal(i18next.t("projects:toast:opening unity..."));
				else toastError("Unity already running");
			}
		}
	};

	let thisDialog: React.JSX.Element | null;
	switch (installStatus.state) {
		case "suggest-unity-hub":
			thisDialog = (
				<UnityInstallWindow
					expectedVersion={installStatus.unityVersion}
					installWithUnityHubLink={installStatus.unityHubLink}
					close={() => setInstallStatus({ state: "normal" })}
				/>
			);
			break;
		case "ask-for-china-version":
			thisDialog = (
				<AskForChinaRevision
					expectedVersion={installStatus.projectUnityVersion}
					chinaUnityVersion={installStatus.chinaUnityVersion}
					useChinaRevision={() => {
						setInstallStatus({ state: "normal" });
						void openUnity(
							installStatus.projectPath,
							installStatus.chinaUnityVersion,
							undefined,
							true,
						);
					}}
					close={() => setInstallStatus({ state: "normal" })}
				/>
			);
			break;
		case "ask-for-international-version":
			thisDialog = (
				<AskForInternationalRevision
					expectedVersion={installStatus.projectUnityVersion}
					internationalUnityVersion={installStatus.internationalUnityVersion}
					useInternationalRevision={() => {
						setInstallStatus({ state: "normal" });
						void openUnity(
							installStatus.projectPath,
							installStatus.internationalUnityVersion,
							undefined,
							true,
						);
					}}
					close={() => setInstallStatus({ state: "normal" })}
				/>
			);
			break;
		case "normal":
			thisDialog = null;
			break;
		default:
			assertNever(installStatus);
	}

	const dialog = (
		<>
			{unitySelector.dialog}
			{thisDialog}
		</>
	);

	return { dialog, openUnity };
}

function UnityInstallWindow({
	expectedVersion,
	installWithUnityHubLink,
	close,
}: {
	expectedVersion: string;
	installWithUnityHubLink: string;
	close: () => void;
}) {
	const openUnityHub = async () => {
		await commands.utilOpenUrl(installWithUnityHubLink);
	};

	return (
		<DialogOpen>
			<DialogTitle>{tc("projects:dialog:unity not found")}</DialogTitle>
			<DialogDescription>
				<p>
					{tc("projects:dialog:unity version of the project not found", {
						unity: expectedVersion,
					})}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={openUnityHub}>
					{tc("projects:dialog:open unity hub")}
				</Button>
				<Button onClick={close} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</DialogOpen>
	);
}

function AskForChinaRevision({
	expectedVersion,
	chinaUnityVersion,
	useChinaRevision,
	close,
}: {
	expectedVersion: string;
	chinaUnityVersion: string;
	useChinaRevision: () => void;
	close: () => void;
}) {
	return (
		<DialogOpen>
			<DialogTitle>
				{tc("projects:dialog:unity not found but china found")}
			</DialogTitle>
			<DialogDescription>
				<p>
					{tc(
						"projects:dialog:unity version of the project not found but china found",
						{
							expectedUnity: expectedVersion,
							chinaUnity: chinaUnityVersion,
						},
					)}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={useChinaRevision}>
					{tc("projects:dialog:use china version")}
				</Button>
				<Button onClick={close} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</DialogOpen>
	);
}

function AskForInternationalRevision({
	expectedVersion,
	internationalUnityVersion,
	useInternationalRevision,
	close,
}: {
	expectedVersion: string;
	internationalUnityVersion: string;
	useInternationalRevision: () => void;
	close: () => void;
}) {
	return (
		<DialogOpen>
			<DialogTitle>
				{tc("projects:dialog:unity not found but international found")}
			</DialogTitle>
			<DialogDescription>
				<p>
					{tc(
						"projects:dialog:unity version of the project not found but international found",
						{
							expectedUnity: expectedVersion,
							internationalUnity: internationalUnityVersion,
						},
					)}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={useInternationalRevision}>
					{tc("projects:dialog:use international version")}
				</Button>
				<Button onClick={close} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</DialogOpen>
	);
}
