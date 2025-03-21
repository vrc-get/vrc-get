import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { UnitySelectorDialog } from "@/components/unity-selector-dialog";
import { type TauriUnityVersions, commands } from "@/lib/bindings";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import i18next, { tc } from "@/lib/i18n";
import { toastError, toastNormal } from "@/lib/toast";
import { parseUnityVersion } from "@/lib/version";

export async function openUnity(
	projectPath: string,
	unityVersion: string | null,
	unityRevision?: string | null,
) {
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

	if (foundVersions.length === 0) {
		// if requested version is not china version and china version is available, suggest to use china version
		// if requested version is china version and international version is available, suggest to use international version
		const askForChina = parseUnityVersion(unityVersion)?.chinaIncrement == null;
		const altVersion = askForChina
			? `${unityVersion}c1`
			: unityVersion.replace(/c\d+$/, "");
		const altInstalls = unityVersions.unity_paths.filter(
			([, v]) => v === altVersion,
		);
		if (altInstalls.length !== 0) {
			if (
				await openSingleDialog(
					askForChina ? AskForChinaRevision : AskForInternationalRevision,
					{
						expectedVersion: unityVersion,
						alternativeVersion: altVersion,
					},
				)
			) {
				await openUnityWith(altInstalls, selectedPath, projectPath);
			}
			return;
		}
		// If there is revision information, we can ask unity hub for install
		if (unityRevision) {
			await openSingleDialog(UnityInstallWindow, {
				expectedVersion: unityVersion,
				installWithUnityHubLink: `unityhub://${unityVersion}/${unityRevision}`,
			});
		} else {
			toastError(
				tc("projects:toast:match version unity not found", {
					unity: unityVersion,
				}),
			);
		}
		return;
	}

	await openUnityWith(foundVersions, selectedPath, projectPath);
}

async function openUnityWith(
	foundVersions: TauriUnityVersions["unity_paths"],
	selectedPath: string | null,
	projectPath: string,
) {
	if (foundVersions.length === 1) {
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
	} else {
		if (selectedPath) {
			const found = foundVersions.find(([p, _v, _i]) => p === selectedPath);
			if (found) {
				const result = await commands.projectOpenUnity(
					projectPath,
					selectedPath,
				);
				if (result) toastNormal(i18next.t("projects:toast:opening unity..."));
				else toastError(i18next.t("projects:toast:unity already running"));
				return;
			}
		}
		const selected = await openSingleDialog(UnitySelectorDialog, {
			unityVersions: foundVersions,
			supportKeepUsing: true,
		});
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

function UnityInstallWindow({
	expectedVersion,
	installWithUnityHubLink,
	dialog,
}: {
	expectedVersion: string;
	installWithUnityHubLink: string;
	dialog: DialogContext<void>;
}) {
	const openUnityHub = async () => {
		await commands.utilOpenUrl(installWithUnityHubLink);
	};

	return (
		<>
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
				<Button onClick={() => dialog.close()} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</>
	);
}

function AskForChinaRevision({
	expectedVersion,
	alternativeVersion,
	dialog,
}: {
	expectedVersion: string;
	alternativeVersion: string;
	dialog: DialogContext<boolean>;
}) {
	return (
		<>
			<DialogTitle>
				{tc("projects:dialog:unity not found but china found")}
			</DialogTitle>
			<DialogDescription>
				<p>
					{tc(
						"projects:dialog:unity version of the project not found but china found",
						{
							expectedUnity: expectedVersion,
							chinaUnity: alternativeVersion,
						},
					)}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={() => dialog.close(true)}>
					{tc("projects:dialog:use china version")}
				</Button>
				<Button onClick={() => dialog.close(false)} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</>
	);
}

function AskForInternationalRevision({
	expectedVersion,
	alternativeVersion,
	dialog,
}: {
	expectedVersion: string;
	alternativeVersion: string;
	dialog: DialogContext<boolean>;
}) {
	return (
		<>
			<DialogTitle>
				{tc("projects:dialog:unity not found but international found")}
			</DialogTitle>
			<DialogDescription>
				<p>
					{tc(
						"projects:dialog:unity version of the project not found but international found",
						{
							expectedUnity: expectedVersion,
							internationalUnity: alternativeVersion,
						},
					)}
				</p>
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={() => dialog.close(true)}>
					{tc("projects:dialog:use international version")}
				</Button>
				<Button onClick={() => dialog.close(false)} className="mr-1">
					{tc("general:button:close")}
				</Button>
			</DialogFooter>
		</>
	);
}
